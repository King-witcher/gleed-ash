use std::time::Instant;

use glam::camera::rh::proj::vulkan::perspective as vulkan_perspective;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec3};

use super::commands::transition_presentation;
use super::uniform::UniformBufferObject;
use crate::allocator::Buffer;
use crate::mesh::Mesh;
use crate::pipeline::Pipeline;
use crate::prelude::*;
use crate::swapchain::{Swapchain, SwapchainImage};

/// Um frame descartado deixa a fence resetada porém nunca submetida, então a
/// próxima espera nessa fence travaria para sempre. Este guard falha alto —
/// exceto durante um unwind, onde um segundo panic viraria abort e esconderia a
/// causa original, que é a que interessa diagnosticar.
///
/// Ele é um campo do [`Frame`] em vez de um `impl Drop for Frame` de propósito:
/// como o `Frame` em si não tem `Drop`, o `submit` pode mover os campos para
/// fora dele.
pub(super) struct MustSubmit;

impl Drop for MustSubmit {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            panic!("Frame was dropped without being submitted");
        }
    }
}

/// Um frame em gravação. Empresta do `Renderer` só o que usa, com o mesmo
/// lifetime: enquanto ele existir, o frame in flight correspondente não pode ser
/// tocado por mais ninguém.
pub struct Frame<'a> {
    pub(super) guard: MustSubmit,
    pub(super) command_buffer: &'a mut vk::raii::CommandBuffer,
    pub(super) ubo: &'a mut Buffer,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) image_available: &'a vk::raii::Semaphore,
    pub(super) fence: &'a vk::raii::Fence,
    pub(super) pipeline: &'a Pipeline,
    pub(super) queue: &'a vk::raii::Queue,
    pub(super) swapchain_image: SwapchainImage,
    pub(super) start_time: Instant,
}

impl Frame<'_> {
    pub fn draw_scene(&mut self, scene: &[Mesh], extent: vk::Extent2D) {
        // Segundos desde o primeiro frame — controla o giro. begin_frame já
        // esperou a fence deste frame, então sobrescrever o UBO aqui não corre
        // risco de corrida com a GPU.
        let time = self.start_time.elapsed().as_secs_f32();

        let aspect = extent.width as f32 / extent.height as f32;

        // Gira 90°/s em torno do eixo (0,1,1). O glm normalizava o eixo por
        // dentro; o glam exige um eixo já normalizado.
        let ubo = UniformBufferObject {
            model: Mat4::from_axis_angle(
                Vec3::new(0.0, 1.0, 1.0).normalize(),
                time * 90.0f32.to_radians(),
            ),
            view: look_at_mat4(
                Vec3::new(0.0, 0.0, 2.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            // Esta projeção já sai com profundidade 0..1 E com Y para baixo, ou
            // seja, embute as duas correções que o C++ fazia à mão:
            // GLM_FORCE_DEPTH_ZERO_TO_ONE e `proj[1][1] *= -1`. A matriz
            // resultante é a mesma (e é por isso que a pipeline usa faces
            // frontais CCW).
            proj: vulkan_perspective(45.0f32.to_radians(), aspect, 0.1, 10.0),
        };

        self.ubo.map_copy_value(&ubo);

        // O command buffer está gravando desde o begin_frame, e a pipeline, o
        // descriptor set e os buffers das meshes vivem mais do que este frame.
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
                    .draw_indexed(mesh.index_count, 1, 0, 0, 0);
            }
        }
    }

    pub fn submit(self, swapchain: &mut Swapchain) -> Result<()> {
        // Desmonta o Frame de uma vez e desarma o guard antes de qualquer `?`:
        // daqui para baixo o frame já é responsabilidade desta função, e se uma
        // chamada falhar o panic do guard mascararia a causa real.
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

        // O buffer continua gravando desde o begin_frame; a imagem é da
        // swapchain, que só é recriada no frame boundary.
        unsafe {
            command_buffer.end_rendering();
            transition_presentation(command_buffer, swapchain_image.image);
            command_buffer.end().context("end the command buffer")?;
        }

        // Trava a escrita na imagem até o acquire_next_image sinalizar. Estágios
        // anteriores (vertex/geometry) podem adiantar enquanto a imagem não chega.
        let wait_semaphores = [vk::SemaphoreSubmitInfo::default()
            .semaphore(image_available.handle())
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];

        // Sinaliza só depois de TUDO, incluindo a transition_presentation() que
        // deixa a imagem em PRESENT_SRC_KHR. Senão o present poderia rodar cedo.
        let signal_semaphores = [vk::SemaphoreSubmitInfo::default()
            .semaphore(swapchain_image.render_finished)
            .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];

        let command_buffer_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer.handle())];

        let submit_info = vk::SubmitInfo2::default()
            .command_buffer_infos(&command_buffer_infos)
            .wait_semaphore_infos(&wait_semaphores)
            .signal_semaphore_infos(&signal_semaphores);

        // A fence foi esperada e resetada no begin_frame deste mesmo frame in
        // flight, e o `Frame` por valor garante que este é o único submit dele.
        unsafe { queue.submit2(&[submit_info], fence.handle()) }
            .context("submit to the graphics queue")?;

        swapchain.present(swapchain_image)
    }
}
