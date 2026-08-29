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
//! [`FrameSlot`] it uses. Recording a frame — including the thousands of
//! `cmd_*` — bumps no counter, and the borrow checker is what guarantees
//! nothing disappears mid-recording.

mod camera;
mod commands;
mod data;
mod frame;
mod frame_slot;
mod pipeline;

use crate::device::Device;
use crate::internal_prelude::*;
use crate::memory::Allocator;
use crate::swapchain::Swapchain;
use commands::{begin_rendering, transition_rendering};
use frame::MustSubmit;
use frame_slot::{make_descriptor_pool, FrameSlot};
use pipeline::Pipeline;

pub use camera::Camera;
pub use frame::Frame;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    slots: [FrameSlot; MAX_FRAMES_IN_FLIGHT],
    _descriptor_pool: vk::raii::DescriptorPool,
    pipeline: Pipeline,
    graphics_queue: vk::raii::Queue,
    _device: Device,
    next_slot: usize,
}

impl Renderer {
    pub fn new(device: Device, allocator: &Allocator, swapchain: &Swapchain) -> Result<Self> {
        let graphics_queue = device.queue(device.graphics_index());
        let pipeline = Pipeline::new(device.clone(), swapchain.image_format())?;
        let descriptor_pool = make_descriptor_pool(&device)?;

        let slots: [FrameSlot; MAX_FRAMES_IN_FLIGHT] = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                FrameSlot::new(
                    &device,
                    allocator,
                    &descriptor_pool,
                    pipeline.descriptor_set_layout(),
                )
            })
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .expect("failed to allocate FrameSlots");

        Ok(Self {
            slots,
            _descriptor_pool: descriptor_pool,
            pipeline,
            graphics_queue,
            _device: device,
            next_slot: 0,
        })
    }

    pub fn begin_frame<'a>(&'a mut self, swapchain: &mut Swapchain) -> Result<Frame<'a>> {
        // Frame boundary: no Frame or SwapchainImage is alive and no semaphore
        // is about to be waited on, so recreating here is safe.
        swapchain.recreate_if_needed()?;

        let slot_index = self.next_slot;
        self.next_slot = (self.next_slot + 1) % MAX_FRAMES_IN_FLIGHT;

        // Borrowed before the slot: they are disjoint fields of the Renderer,
        // so they coexist with the `&mut self.slots` below.
        let pipeline = &self.pipeline;
        let queue = &self.graphics_queue;

        let slot = &mut self.slots[slot_index];

        unsafe {
            slot.fence
                .wait(u64::MAX)
                .context("wait for the frame fence")?;
            slot.fence.reset().context("reset the frame fence")?;
        }

        let swapchain_image = swapchain.acquire_next_image(slot.image_available.handle())?;

        let extent = swapchain.extent();
        let command_buffer = &mut slot.command_buffer;

        // The fence above already guaranteed the GPU is done with this pool,
        // and the `&mut self` that nothing else is recording into it.
        unsafe {
            slot.command_pool
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
            camera_buffer: &mut slot.camera_buffer,
            descriptor_set: slot.descriptor_set,
            image_available: &slot.image_available,
            fence: &slot.fence,
            pipeline,
            queue,
            swapchain_image,
        })
    }
}
