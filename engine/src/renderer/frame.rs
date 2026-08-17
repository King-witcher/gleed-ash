use std::time::Instant;

use glam::{Mat4, Vec3};

use super::commands::transition_presentation;
use super::pipeline::Pipeline;
use super::uniform::Transforms;
use crate::internal_prelude::*;
use crate::memory::{Buffer, HostVisible};
use crate::mesh::Mesh;
use crate::renderer::Camera;
use crate::swapchain::{Swapchain, SwapchainImage};

/// A dropped frame leaves the fence reset but never submitted, so the next
/// wait on that fence would block forever. This guard fails loudly — except
/// during an unwind, where a second panic would become an abort and hide the
/// original cause, which is the one worth diagnosing.
///
/// It is a field of [`Frame`] instead of an `impl Drop for Frame` on purpose:
/// since `Frame` itself has no `Drop`, `submit` can move the fields out of it.
pub(super) struct MustSubmit;

impl Drop for MustSubmit {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            panic!("Frame was dropped without being submitted");
        }
    }
}

/// A frame being recorded. Borrows from the `Renderer` only what it uses, with
/// the same lifetime: while it exists, the matching [`FrameSlot`] cannot be
/// touched by anyone else.
///
/// [`FrameSlot`]: super::FrameSlot
pub struct Frame<'a> {
    pub(super) guard: MustSubmit,
    pub(super) command_buffer: &'a mut vk::raii::CommandBuffer,
    pub(super) transforms_buffer: &'a mut Buffer<HostVisible>,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) image_available: &'a vk::raii::Semaphore,
    pub(super) fence: &'a vk::raii::Fence,
    pub(super) pipeline: &'a Pipeline,
    pub(super) queue: &'a vk::raii::Queue,
    pub(super) swapchain_image: SwapchainImage,
    pub(super) start_time: Instant,
}

impl Frame<'_> {
    pub fn set_camera(&mut self, camera: &Camera) {
        let transforms = Transforms {
            model: Mat4::from_axis_angle(Vec3::new(0.0, 1.0, 1.0).normalize(), 0.0),
            camera: camera.matrix(),
        };

        self.transforms_buffer.map_copy_value(&transforms);
    }

    pub fn draw_scene(&mut self, scene: &[Mesh]) {
        // The command buffer has been recording since begin_frame, and the
        // pipeline, descriptor set and mesh buffers outlive this frame.
        unsafe {
            self.command_buffer
                .bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.vk_pipeline());
            self.command_buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout(),
                0,
                &[self.descriptor_set],
                &[],
            );

            for mesh in scene {
                self.command_buffer.bind_vertex_buffers(
                    0,
                    &[mesh.vertex_buffer().vk_buffer()],
                    &[0],
                );
                self.command_buffer.bind_index_buffer(
                    mesh.index_buffer().vk_buffer(),
                    0,
                    vk::IndexType::UINT32,
                );
                self.command_buffer
                    .draw_indexed(mesh.index_count(), 1, 0, 0, 0);
            }
        }
    }

    pub fn submit(self, swapchain: &mut Swapchain) -> Result<()> {
        // Dismantle the Frame at once and disarm the guard before any `?`:
        // from here on the frame is this function's responsibility, and if a
        // call failed the guard's panic would mask the real cause.
        let Frame {
            guard,
            command_buffer,
            image_available,
            fence,
            queue,
            swapchain_image,
            ..
        } = self;
        std::mem::forget(guard);

        // The buffer has been recording since begin_frame; the image belongs
        // to the swapchain, which is only recreated at the frame boundary.
        unsafe {
            command_buffer.end_rendering();
            transition_presentation(command_buffer, swapchain_image.image);
            command_buffer.end().context("end the command buffer")?;
        }

        // Holds writes to the image until acquire_next_image signals. Earlier
        // stages (vertex/geometry) may run ahead while the image is not there.
        let wait_semaphores = [vk::SemaphoreSubmitInfo::default()
            .semaphore(image_available.handle())
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];

        // Signals only after EVERYTHING, including the
        // transition_presentation() that leaves the image in PRESENT_SRC_KHR.
        // Otherwise present could run early.
        let signal_semaphores = [vk::SemaphoreSubmitInfo::default()
            .semaphore(swapchain_image.render_finished)
            .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];

        let command_buffer_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer.handle())];

        let submit_info = vk::SubmitInfo2::default()
            .command_buffer_infos(&command_buffer_infos)
            .wait_semaphore_infos(&wait_semaphores)
            .signal_semaphore_infos(&signal_semaphores);

        // The fence was waited on and reset in this same slot's begin_frame,
        // and the by-value `Frame` guarantees this is its only submit.
        unsafe { queue.submit2(&[submit_info], fence.handle()) }
            .context("submit to the graphics queue")?;

        swapchain.present(swapchain_image)
    }
}
