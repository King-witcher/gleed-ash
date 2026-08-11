//! Comandos de render pass compartilhados entre o início do frame
//! ([`super::Renderer::begin_frame`]) e o seu fim ([`super::Frame::submit`]).
//!
//! Todas as funções são `unsafe` pelo mesmo motivo: o `vk::raii` não checa a
//! máquina de estados do command buffer, então quem chama é que garante que ele
//! está gravando e que a imagem passada continua viva.

use crate::swapchain::SwapchainImage;

/// # Safety
/// Ver a nota do módulo.
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
        // O retângulo da imagem que este render pass deve afetar.
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        })
        .layer_count(1)
        .color_attachments(&color_attachments);

    unsafe {
        command_buffer.begin_rendering(&rendering_info);

        // Define o tamanho do container dentro do qual a imagem renderizada será
        // encaixada.
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
/// Ver a nota do módulo.
pub(super) unsafe fn transition_rendering(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
) {
    let barriers = [vk::ImageMemoryBarrier2::default()
        // O que a transição deve esperar antes de rodar. Mesmo não havendo
        // comandos antes na pipeline, cria uma dependência no estágio de
        // Color Attachment Output. Isso impede a transição de acontecer antes
        // do semáforo imageAvailable, que trava esse estágio, sinalizar.
        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        // Não há escritas de memória a tornar disponíveis.
        .src_access_mask(vk::AccessFlags2::empty())
        // O que deve esperar a transição antes de rodar.
        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        // Torna este acesso visível (invalida cache)
        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        // Como usamos a mesma queue para tudo, nada precisa ser transferido.
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(color_subresource_range())];

    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
    unsafe { command_buffer.pipeline_barrier2(&dependency_info) };
}

/// # Safety
/// Ver a nota do módulo.
pub(super) unsafe fn transition_presentation(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
) {
    let barriers = [vk::ImageMemoryBarrier2::default()
        // Espera todos os Color Attachment Outputs terminarem antes de voltar
        // para o layout present optimal.
        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        // Faz flush das escritas de color attachment.
        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        // Não há nada neste buffer esperando esta barreira terminar.
        // Equivalente a eBottomOfPipe.
        .dst_stage_mask(vk::PipelineStageFlags2::empty())
        // Nada a tornar disponível.
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
