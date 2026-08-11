//! Equivalente a modules/engine/src/renderer.{h,cc}.
//!
//! O [`Frame`] é um "token linear" que precisa ser consumido por `submit`
//! exatamente uma vez. No C++ isso era simulado com construtor de move +
//! destrutor que dá panic. Em Rust o próprio sistema de tipos faz metade do
//! trabalho: `submit` recebe o `Frame` **por valor**, então usá-lo depois é erro
//! de compilação, não de runtime.
//!
//! Regra de custo do módulo: **refcount na hierarquia grossa, empréstimo na
//! gravação**. Device, command pool, pipeline e buffers são refcontados uma vez,
//! na criação; o `Frame` só carrega referências para as partes do frame in
//! flight que usa. Gravar um frame — inclusive os milhares de `cmd_*` — não
//! incrementa contador nenhum, e o borrow checker é quem garante que nada some
//! no meio da gravação.

mod commands;
mod frame;
mod frame_in_flight;
mod uniform;

use std::rc::Rc;
use std::time::Instant;

use crate::allocator::Allocator;
use crate::device::Device;
use crate::pipeline::Pipeline;
use crate::prelude::*;
use crate::swapchain::Swapchain;
use commands::{begin_rendering, transition_rendering};
use frame::MustSubmit;
use frame_in_flight::{make_descriptor_pool, FrameInFlight};

pub use frame::Frame;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    // A ORDEM DOS CAMPOS É A ORDEM DE DESTRUIÇÃO: os frames usam descriptor sets
    // do pool abaixo, e destruir o pool já os libera — não é preciso o
    // FREE_DESCRIPTOR_SET que o C++ usava (lá cada vk::raii::DescriptorSet se
    // liberava individualmente).
    frames: Vec<FrameInFlight>,
    descriptor_pool: vk::raii::DescriptorPool,
    pipeline: Pipeline,
    graphics_queue: vk::raii::Queue,
    #[allow(dead_code)]
    allocator: Rc<Allocator>,
    device: Device,
    next_frame: usize,
    start_time: Instant,
}

impl Renderer {
    pub fn new(device: Device, allocator: Rc<Allocator>, swapchain: &Swapchain) -> Result<Self> {
        let graphics_queue = device.get_queue(device.graphics_index());
        let pipeline = Pipeline::new(device.clone(), swapchain.image_format())?;
        let descriptor_pool = make_descriptor_pool(&device)?;

        let frames = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| {
                FrameInFlight::new(
                    &device,
                    &allocator,
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
            allocator,
            device,
            next_frame: 0,
            start_time: Instant::now(),
        })
    }

    pub fn begin_frame<'a>(&'a mut self, swapchain: &mut Swapchain) -> Result<Frame<'a>> {
        // Frame boundary: não existe nenhum Frame ou SwapchainImage vivo e
        // nenhum semáforo está prestes a ser esperado, então recriar aqui é seguro.
        swapchain.recreate_if_needed()?;

        let frame_index = self.next_frame;
        self.next_frame = (self.next_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        // Emprestados antes do frame in flight: são campos disjuntos do
        // Renderer, então convivem com o `&mut self.frames` de baixo.
        let start_time = self.start_time;
        let pipeline = &self.pipeline;
        let queue = &self.graphics_queue;

        let frame = &mut self.frames[frame_index];

        // A fence só é esperada aqui e só é sinalizada pelo submit deste mesmo
        // frame in flight, então nada mais depende dela neste ponto.
        unsafe {
            frame
                .fence
                .wait(u64::MAX)
                .context("esperar a fence do frame")?;
            frame.fence.reset().context("resetar a fence do frame")?;
        }

        let swapchain_image = swapchain.acquire_next_image(frame.image_available.handle())?;

        let extent = swapchain.extent();
        let command_buffer = &mut frame.command_buffer;

        // A fence acima já garantiu que a GPU terminou com este pool, e o
        // `&mut self` que ninguém mais está gravando nele.
        unsafe {
            frame
                .command_pool
                .reset(vk::CommandPoolResetFlags::empty())
                .context("resetar o command pool do frame")?;

            command_buffer
                .begin(&vk::CommandBufferBeginInfo::default())
                .context("iniciar o command buffer")?;

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
