//! Equivalent to modules/engine/src/renderer.{h,cc}.
//!
//! [`Frame`] is a "linear token" that must be consumed by `submit` exactly
//! once. C++ simulated that with a move constructor + a panicking destructor.
//! In Rust the type system does half the work: `submit` takes the `Frame`
//! **by value**, so using it afterwards is a compile error, not a runtime one.
//!
//! The module's cost rule: **refcount on the coarse hierarchy, borrows while
//! recording**. Device, command pool, pipeline and buffers are refcounted
//! once, at creation; the `Frame` only carries references to the parts of the
//! frame in flight it uses. Recording a frame — including the thousands of
//! `cmd_*` — bumps no counter, and the borrow checker is what guarantees
//! nothing disappears mid-recording.

mod commands;
mod frame;
mod frame_in_flight;
mod pipeline;
mod uniform;

use std::time::Instant;

use crate::device::Device;
use crate::memory::Allocator;
use crate::prelude::*;
use crate::swapchain::Swapchain;
use commands::{begin_rendering, transition_rendering};
use frame::MustSubmit;
use frame_in_flight::{make_descriptor_pool, FrameInFlight};
use pipeline::Pipeline;

pub use frame::Frame;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    // FIELD ORDER IS DESTRUCTION ORDER: the frames use descriptor sets from
    // the pool below, and destroying the pool releases them — no need for the
    // FREE_DESCRIPTOR_SET the C++ used (there each vk::raii::DescriptorSet
    // freed itself individually).
    frames: Vec<FrameInFlight>,
    descriptor_pool: vk::raii::DescriptorPool,
    pipeline: Pipeline,
    graphics_queue: vk::raii::Queue,
    device: Device,
    next_frame: usize,
    start_time: Instant,
}

impl Renderer {
    pub fn new(device: Device, allocator: &Allocator, swapchain: &Swapchain) -> Result<Self> {
        let graphics_queue = device.queue(device.graphics_index());
        let pipeline = Pipeline::new(device.clone(), swapchain.image_format())?;
        let descriptor_pool = make_descriptor_pool(&device)?;

        let frames = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                FrameInFlight::new(
                    &device,
                    allocator,
                    &descriptor_pool,
                    pipeline.descriptor_set_layout(),
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            frames,
            descriptor_pool,
            pipeline,
            graphics_queue,
            device,
            next_frame: 0,
            start_time: Instant::now(),
        })
    }

    pub fn begin_frame<'a>(&'a mut self, swapchain: &mut Swapchain) -> Result<Frame<'a>> {
        // Frame boundary: no Frame or SwapchainImage is alive and no semaphore
        // is about to be waited on, so recreating here is safe.
        swapchain.recreate_if_needed()?;

        let frame_index = self.next_frame;
        self.next_frame = (self.next_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        // Borrowed before the frame in flight: they are disjoint fields of the
        // Renderer, so they coexist with the `&mut self.frames` below.
        let start_time = self.start_time;
        let pipeline = &self.pipeline;
        let queue = &self.graphics_queue;

        let frame = &mut self.frames[frame_index];

        // The fence is only waited on here and only signaled by this same
        // frame in flight's submit, so nothing else depends on it at this
        // point.
        unsafe {
            frame
                .fence
                .wait(u64::MAX)
                .context("wait for the frame fence")?;
            frame.fence.reset().context("reset the frame fence")?;
        }

        let swapchain_image = swapchain.acquire_next_image(frame.image_available.handle())?;

        let extent = swapchain.extent();
        let command_buffer = &mut frame.command_buffer;

        // The fence above already guaranteed the GPU is done with this pool,
        // and the `&mut self` that nothing else is recording into it.
        unsafe {
            frame
                .command_pool
                .reset(vk::CommandPoolResetFlags::empty())
                .context("reset the frame command pool")?;

            command_buffer
                .begin(&vk::CommandBufferBeginInfo::default())
                .context("begin the command buffer")?;

            transition_rendering(command_buffer, swapchain_image.image);
            begin_rendering(command_buffer, &swapchain_image, extent);
        }

        Ok(Frame {
            guard: MustSubmit,
            command_buffer,
            ubo: &mut frame.ubo,
            descriptor_set: frame.descriptor_set,
            image_available: &frame.image_available,
            fence: &frame.fence,
            pipeline,
            queue,
            swapchain_image,
            start_time,
        })
    }
}
