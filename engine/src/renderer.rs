//! Equivalente a modules/engine/src/renderer.{h,cc}.
//!
//! O `RenderPass` continua sendo um "token linear" que precisa ser consumido
//! por `submit_frame` exatamente uma vez. No C++ isso era simulado com
//! construtor de move + destrutor que dá Panic. Em Rust o próprio sistema de
//! tipos faz metade do trabalho: `submit_frame` recebe o `RenderPass` **por
//! valor**, então usá-lo depois é erro de compilação, não de runtime.

use std::rc::Rc;
use std::time::Instant;

use ash::vk;
use glam::camera::rh::proj::vulkan::perspective as vulkan_perspective;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec3};

use crate::allocator::{AllocMode, Allocator, Buffer};
use crate::device::Device;
use crate::mesh::Mesh;
use crate::pipeline::Pipeline;
use crate::prelude::*;
use crate::swapchain::{Swapchain, SwapchainImage};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// Transformações por frame enviadas ao shader. O layout precisa casar com o
/// `UniformBuffer` em shaders/mesh.slang (três float4x4 std140; o `Mat4` do
/// glam, como o do glm, já casa).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

struct FrameInFlight {
    ubo: Buffer,
    command_buffer: vk::CommandBuffer,
    descriptor_set: vk::DescriptorSet,
    /// Ambos são RAII e carregam um clone do device, então somem junto com o
    /// frame — o `Drop` do `Renderer` não precisa destruí-los à mão.
    image_available: vk_raii::Semaphore,
    fence: vk_raii::Fence,
}

impl FrameInFlight {
    fn new(
        device: &Device,
        allocator: &Allocator,
        pool: vk::CommandPool,
        descriptor_pool: vk::DescriptorPool,
        layout: vk::DescriptorSetLayout,
    ) -> Result<Self> {
        let ubo = make_ubo(allocator)?;
        let command_buffer = make_command_buffer(device, pool)?;
        let descriptor_set = make_descriptor_set(device, descriptor_pool, layout)?;

        // Aponta o descriptor set deste frame para o seu próprio UBO. O buffer é
        // persistente, então esse binding é escrito uma vez e só o conteúdo muda
        // por frame — não é preciso reescrever o descriptor set todo frame.
        let buffer_infos = [vk::DescriptorBufferInfo::default()
            .buffer(ubo.vk_buffer())
            .offset(0)
            .range(std::mem::size_of::<UniformBufferObject>() as u64)];

        let writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_infos)];

        unsafe { device.raw().update_descriptor_sets(&writes, &[]) };

        Ok(Self {
            ubo,
            command_buffer,
            descriptor_set,
            image_available: device.create_semaphore()?,
            fence: device.create_fence(true)?,
        })
    }
}

/// Os `vkCmd*` são comandos device-level: os ponteiros vêm de
/// `vkGetDeviceProcAddr`, então quem sabe despachá-los é o device — não há como
/// gravar comandos sem ele por perto. Isso é o mesmo que o C++ faz, só que
/// escondido: o `vk::raii::CommandBuffer` carrega um `m_dispatcher` dentro de
/// cada objeto. Aqui o compartilhamento é por refcount, não por valor:
/// `ash::Device` é a tabela de dispatch inteira (~185 ponteiros de função) e o
/// seu `Clone` copia tudo, o que sairia como um memcpy de ~1,5 KB por frame.
pub struct RenderPass {
    device: Device,
    command_buffer: vk::CommandBuffer,
    frame_index: usize,
    swapchain_image: SwapchainImage,
    submitted: bool,
}

impl RenderPass {
    pub fn bind_pipeline(&mut self, pipeline: &Pipeline) {
        unsafe {
            self.device.raw().cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.vk_pipeline(),
            )
        };
    }

    pub fn bind_descriptor_set(&mut self, layout: vk::PipelineLayout, set: vk::DescriptorSet) {
        unsafe {
            self.device.raw().cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                layout,
                0,
                &[set],
                &[],
            )
        };
    }

    pub fn bind_vertex_buffer(&mut self, buffer: &Buffer) {
        unsafe {
            self.device.raw().cmd_bind_vertex_buffers(
                self.command_buffer,
                0,
                &[buffer.vk_buffer()],
                &[0],
            )
        };
    }

    pub fn bind_index_buffer(&mut self, buffer: &Buffer) {
        unsafe {
            self.device.raw().cmd_bind_index_buffer(
                self.command_buffer,
                buffer.vk_buffer(),
                0,
                vk::IndexType::UINT32,
            )
        };
    }

    pub fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.raw().cmd_draw(
                self.command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        };
    }

    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.raw().cmd_draw_indexed(
                self.command_buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            )
        };
    }

    fn begin_rendering(&mut self, extent: vk::Extent2D) {
        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(self.swapchain_image.image_view)
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
            self.device
                .raw()
                .cmd_begin_rendering(self.command_buffer, &rendering_info);

            // Define o tamanho do container dentro do qual a imagem renderizada
            // será encaixada.
            self.device.raw().cmd_set_viewport(
                self.command_buffer,
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

            self.device.raw().cmd_set_scissor(
                self.command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                }],
            );
        }
    }

    fn end_rendering(&mut self) -> Result<()> {
        unsafe { self.device.raw().cmd_end_rendering(self.command_buffer) };
        self.transition_presentation();
        unsafe { self.device.raw().end_command_buffer(self.command_buffer) }
            .context("finalizar o command buffer")?;
        Ok(())
    }

    fn transition_rendering(&mut self) {
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
            .image(self.swapchain_image.image)
            .subresource_range(color_subresource_range())];

        let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
        unsafe {
            self.device
                .raw()
                .cmd_pipeline_barrier2(self.command_buffer, &dependency_info)
        };
    }

    fn transition_presentation(&mut self) {
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
            .image(self.swapchain_image.image)
            .subresource_range(color_subresource_range())];

        let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
        unsafe {
            self.device
                .raw()
                .cmd_pipeline_barrier2(self.command_buffer, &dependency_info)
        };
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        // Um RenderPass descartado deixa a fence resetada porém nunca submetida,
        // então a próxima espera nessa fence travaria para sempre. Falha alto —
        // exceto durante um unwind, onde um segundo panic viraria abort e
        // esconderia a causa original, que é a que interessa diagnosticar.
        if !self.submitted && !std::thread::panicking() {
            panic!("RenderPass was dropped without being submitted");
        }
    }
}

pub struct Renderer {
    device: Device,
    #[allow(dead_code)]
    allocator: Rc<Allocator>,
    command_pool: vk::CommandPool,
    graphics_queue: vk_raii::Queue,
    pipeline: Pipeline,
    descriptor_pool: vk::DescriptorPool,
    frames: Vec<FrameInFlight>,
    next_frame: usize,
    start_time: Instant,
}

impl Renderer {
    pub fn new(device: Device, allocator: Rc<Allocator>, swapchain: &Swapchain) -> Result<Self> {
        let command_pool = make_command_pool(&device)?;
        let graphics_queue = device.get_queue(device.graphics_index());
        let pipeline = Pipeline::new(device.clone(), swapchain.image_format())?;
        let descriptor_pool = make_descriptor_pool(&device)?;

        let frames = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                FrameInFlight::new(
                    &device,
                    &allocator,
                    command_pool,
                    descriptor_pool,
                    pipeline.descriptor_set_layout(),
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            device,
            allocator,
            command_pool,
            graphics_queue,
            pipeline,
            descriptor_pool,
            frames,
            next_frame: 0,
            start_time: Instant::now(),
        })
    }

    pub fn begin_render_pass(&mut self, swapchain: &mut Swapchain) -> Result<RenderPass> {
        // Frame boundary: não existe nenhum RenderPass ou SwapchainImage vivo e
        // nenhum semáforo está prestes a ser esperado, então recriar aqui é seguro.
        swapchain.recreate_if_needed()?;

        let frame_index = self.next_frame;
        let frame = &self.frames[frame_index];
        let command_buffer = frame.command_buffer;

        frame
            .fence
            .wait_and_reset()
            .context("esperar a fence do frame")?;
        let swapchain_image = swapchain.acquire_next_image(frame.image_available.handle())?;

        let raw = self.device.raw();
        unsafe {
            raw.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .context("resetar o command buffer")?;
            raw.begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())
                .context("iniciar o command buffer")?;
        }

        let mut render_pass = RenderPass {
            device: self.device.clone(),
            command_buffer,
            frame_index,
            swapchain_image,
            submitted: false,
        };
        render_pass.transition_rendering();
        render_pass.begin_rendering(swapchain.extent());

        self.next_frame = (self.next_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(render_pass)
    }

    pub fn draw_scene(&mut self, frame: &mut RenderPass, scene: &[Mesh], extent: vk::Extent2D) {
        // Segundos desde o primeiro frame — controla o giro. begin_render_pass já
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

        let descriptor_set = {
            let frame_in_flight = &mut self.frames[frame.frame_index];
            frame_in_flight.ubo.map_copy_value(&ubo);
            frame_in_flight.descriptor_set
        };

        frame.bind_pipeline(&self.pipeline);
        frame.bind_descriptor_set(self.pipeline.layout(), descriptor_set);

        for mesh in scene {
            frame.bind_vertex_buffer(mesh.vertex_buffer());
            frame.bind_index_buffer(mesh.index_buffer());
            frame.draw_indexed(mesh.index_count, 1, 0, 0, 0);
        }
    }

    pub fn submit_frame(
        &mut self,
        swapchain: &mut Swapchain,
        mut render_pass: RenderPass,
    ) -> Result<()> {
        // Marcado antes de qualquer `?`: daqui para baixo o RenderPass já é
        // responsabilidade desta função, e se uma chamada falhar ele é descartado
        // no caminho do erro — o panic do Drop mascararia a causa real.
        render_pass.submitted = true;

        render_pass.end_rendering()?;

        let frame = &self.frames[render_pass.frame_index];
        let render_finished = render_pass.swapchain_image.render_finished;

        // Trava a escrita na imagem até o acquire_next_image sinalizar. Estágios
        // anteriores (vertex/geometry) podem adiantar enquanto a imagem não chega.
        let wait_semaphores = [vk::SemaphoreSubmitInfo::default()
            .semaphore(frame.image_available.handle())
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];

        // Sinaliza só depois de TUDO, incluindo a transition_presentation() que
        // deixa a imagem em PRESENT_SRC_KHR. Senão o present poderia rodar cedo.
        let signal_semaphores = [vk::SemaphoreSubmitInfo::default()
            .semaphore(render_finished)
            .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];

        let command_buffer_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(render_pass.command_buffer)];

        let submit_info = vk::SubmitInfo2::default()
            .command_buffer_infos(&command_buffer_infos)
            .wait_semaphore_infos(&wait_semaphores)
            .signal_semaphore_infos(&signal_semaphores);

        // `unsafe` porque o command buffer ainda é um handle cru: nada garante
        // por tipo que ele esteja gravado e fora de uso.
        unsafe {
            self.graphics_queue
                .submit2(&[submit_info], Some(&frame.fence))
        }
        .context("submeter para a graphics queue")?;

        swapchain.present(render_pass.swapchain_image)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let raw = self.device.raw();
        unsafe {
            // Destruir o pool já libera os descriptor sets, então aqui não é
            // preciso o FREE_DESCRIPTOR_SET que o C++ usava (lá cada
            // vk::raii::DescriptorSet se liberava individualmente).
            raw.destroy_descriptor_pool(self.descriptor_pool, None);
            raw.destroy_command_pool(self.command_pool, None);
        }
    }
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

fn make_command_pool(device: &Device) -> Result<vk::CommandPool> {
    let info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(device.graphics_index());

    unsafe { device.raw().create_command_pool(&info, None) }.context("criar command pool")
}

fn make_descriptor_pool(device: &Device) -> Result<vk::DescriptorPool> {
    // Um descriptor de uniform buffer por frame in flight.
    let pool_sizes = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32)];

    let info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    unsafe { device.raw().create_descriptor_pool(&info, None) }.context("criar descriptor pool")
}

fn make_command_buffer(device: &Device, pool: vk::CommandPool) -> Result<vk::CommandBuffer> {
    let info = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .command_buffer_count(1)
        .level(vk::CommandBufferLevel::PRIMARY);

    let buffers =
        unsafe { device.raw().allocate_command_buffers(&info) }.context("alocar command buffer")?;

    Ok(buffers[0])
}

fn make_ubo(allocator: &Allocator) -> Result<Buffer> {
    let info = vk::BufferCreateInfo::default()
        .size(std::mem::size_of::<UniformBufferObject>() as u64)
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    // HostVisible: a CPU reescreve as transformações todo frame.
    allocator.allocate(&info, AllocMode::HostVisible)
}

fn make_descriptor_set(
    device: &Device,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> Result<vk::DescriptorSet> {
    let set_layouts = [layout];
    let info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(pool)
        .set_layouts(&set_layouts);

    let sets =
        unsafe { device.raw().allocate_descriptor_sets(&info) }.context("alocar descriptor set")?;

    Ok(sets[0])
}
