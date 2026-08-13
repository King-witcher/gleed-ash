//! Render pass commands shared between the start of the frame
//! ([`super::Renderer::begin_frame`]) and its end ([`super::Frame::submit`]).
//!
//! Every function is `unsafe` for the same reason: `vk::raii` does not check
//! the command buffer state machine, so the caller guarantees it is recording
//! and that the image passed in stays alive.

use crate::swapchain::SwapchainImage;

/// # Safety
/// See the module note.
pub(super) unsafe fn begin_rendering(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: &SwapchainImage,
    extent: vk::Extent2D,
) {
    let color_attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(image.image_view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.05, 0.05, 0.05, 1.0],
            },
        })];

    let rendering_info = vk::RenderingInfo::default()
        // The rectangle of the image this render pass affects.
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        })
        .layer_count(1)
        .color_attachments(&color_attachments);

    unsafe {
        command_buffer.begin_rendering(&rendering_info);

        // Sets the size of the container the rendered image is fitted into.
        command_buffer.set_viewport(
            0,
            &[vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );

        command_buffer.set_scissor(
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            }],
        );
    }
}

/// # Safety
/// See the module note.
pub(super) unsafe fn transition_rendering(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
) {
    let barriers = [vk::ImageMemoryBarrier2::default()
        // What the transition must wait for before running. Even with no
        // earlier commands in the pipeline, this creates a dependency on the
        // Color Attachment Output stage. It keeps the transition from
        // happening before the imageAvailable semaphore — which gates that
        // stage — signals.
        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        // No memory writes to make available.
        .src_access_mask(vk::AccessFlags2::empty())
        // What must wait for the transition before running.
        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        // Makes this access visible (invalidates caches).
        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        // Everything runs on the same queue, so nothing needs transferring.
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(color_subresource_range())];

    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
    unsafe { command_buffer.pipeline_barrier2(&dependency_info) };
}

/// # Safety
/// See the module note.
pub(super) unsafe fn transition_presentation(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
) {
    let barriers = [vk::ImageMemoryBarrier2::default()
        // Waits for every Color Attachment Output to finish before moving back
        // to the present optimal layout.
        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        // Flushes the color attachment writes.
        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        // Nothing in this buffer waits for this barrier to finish.
        // Equivalent to eBottomOfPipe.
        .dst_stage_mask(vk::PipelineStageFlags2::empty())
        // Nothing to make available.
        .dst_access_mask(vk::AccessFlags2::empty())
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(color_subresource_range())];

    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
    unsafe { command_buffer.pipeline_barrier2(&dependency_info) };
}

fn color_subresource_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}
