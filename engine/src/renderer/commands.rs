use crate::swapchain::SwapchainImage;

pub(super) unsafe fn begin_rendering(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: &SwapchainImage,
    extent: vk::Extent2D,
) {
    let color_attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(image.image_view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
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

pub(super) unsafe fn transition_rendering(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
) {
    let barriers = [vk::ImageMemoryBarrier2::default()
        // src
        .old_layout(vk::ImageLayout::UNDEFINED)
        .src_stage_mask(vk::PipelineStageFlags2::NONE)
        .src_access_mask(vk::AccessFlags2::empty())
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        // dst
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        // etc
        .image(image)
        .subresource_range(color_subresource_range())];

    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
    unsafe { command_buffer.pipeline_barrier2(&dependency_info) };
}

pub(super) unsafe fn transition_presentation(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
) {
    let barriers = [vk::ImageMemoryBarrier2::default()
        // src
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        // dst
        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .dst_stage_mask(vk::PipelineStageFlags2::empty())
        .dst_access_mask(vk::AccessFlags2::empty())
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        // etc
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
