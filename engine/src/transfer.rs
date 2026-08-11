//! Equivalente a modules/engine/src/transfer.{h,cc}.

use std::rc::Rc;

use crate::allocator::{as_bytes, AllocMode, Allocator, Buffer};
use crate::device::Device;
use crate::prelude::*;

pub struct TransferContext {
    allocator: Rc<Allocator>,
    queue: vk::raii::Queue,
    /// Um único command buffer reciclado a cada upload: o pool é resetado antes
    /// de gravar, em vez de alocar e liberar um buffer por vez.
    command_buffer: vk::raii::CommandBuffer,
    pool: vk::raii::CommandPool,
    fence: vk::raii::Fence,
}

impl TransferContext {
    pub fn new(device: Device, allocator: Rc<Allocator>) -> Result<Self> {
        let queue = device.get_queue(device.graphics_index());
        let pool = make_command_pool(&device)?;
        let command_buffer = unsafe { pool.allocate_one(vk::CommandBufferLevel::PRIMARY) }
            .context("alocar transfer command buffer")?;
        // Não sinalizada: a primeira espera precisa realmente esperar a GPU. Se
        // começasse sinalizada, ela retornaria antes da cópia terminar.
        let fence = device.create_fence(false)?;

        Ok(Self {
            allocator,
            queue,
            command_buffer,
            pool,
            fence,
        })
    }

    /// Grava um closure no command buffer do contexto, submete e BLOQUEIA até a
    /// GPU terminar. Usado para uploads, transições de layout, etc.
    ///
    /// `&mut self` porque gravar exige acesso exclusivo ao command buffer — é a
    /// mesma regra do Vulkan, só que checada pelo compilador.
    pub fn immediate_submit(
        &mut self,
        record: impl FnOnce(&mut vk::raii::CommandBuffer),
    ) -> Result<()> {
        // A espera da fence no fim da chamada anterior já garantiu que a GPU
        // terminou com este buffer, e o `&mut self` que nada mais está gravando.
        unsafe { self.pool.reset(vk::CommandPoolResetFlags::empty()) }
            .context("resetar o transfer command pool")?;

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.command_buffer.begin(&begin_info) }
            .context("iniciar transfer command buffer")?;

        record(&mut self.command_buffer);

        unsafe { self.command_buffer.end() }.context("finalizar transfer command buffer")?;

        let cmd_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(self.command_buffer.handle())];
        let submit_info = vk::SubmitInfo2::default().command_buffer_infos(&cmd_infos);

        // A fence está livre: a chamada anterior esperou e resetou ela, e o
        // buffer acabou de sair do `end`.
        unsafe { self.queue.submit2(&[submit_info], self.fence.handle()) }
            .context("submeter transfer command buffer")?;

        // Bloqueia até a GPU terminar e deixa a fence pronta para o próximo uso.
        unsafe { self.fence.wait(u64::MAX) }.context("esperar a transferência terminar")?;
        unsafe { self.fence.reset() }.context("resetar a fence de transferência")
    }

    /// Aloca um buffer device-local (VRAM) e sobe `data` para ele via staging.
    /// O buffer de staging host-visible é criado e liberado internamente.
    pub fn upload_buffer(&mut self, data: &[u8], usage: vk::BufferUsageFlags) -> Result<Buffer> {
        // 1. Staging host-visible: a CPU escreve os dados aqui.
        let staging_info = vk::BufferCreateInfo::default()
            .size(data.len() as u64)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let mut staging = self
            .allocator
            .allocate(&staging_info, AllocMode::HostVisible)?;
        staging.map_copy(data);

        // 2. Destino device-local (VRAM), também alvo de transferência.
        let gpu_info = vk::BufferCreateInfo::default()
            .size(data.len() as u64)
            .usage(usage | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let gpu = self.allocator.allocate(&gpu_info, AllocMode::DeviceLocal)?;

        // 3. Copia staging -> device-local na GPU e espera a cópia terminar.
        self.immediate_submit(|command_buffer| {
            let region = vk::BufferCopy::default().size(data.len() as u64);
            // O buffer está gravando (immediate_submit acabou de dar begin) e os
            // dois buffers estão vivos até o fim desta função.
            unsafe { command_buffer.copy_buffer(staging.vk_buffer(), gpu.vk_buffer(), &[region]) };
        })?;

        // `staging` é destruído aqui — seguro, já que immediate_submit esperou a
        // fence, garantindo que a cópia terminou.
        Ok(gpu)
    }

    pub fn upload_slice<T: Copy>(
        &mut self,
        values: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer> {
        self.upload_buffer(as_bytes(values), usage)
    }
}

fn make_command_pool(device: &Device) -> Result<vk::raii::CommandPool> {
    let info = vk::CommandPoolCreateInfo::default()
        // TRANSIENT: o conteúdo gravado é efêmero — um upload e o pool é resetado.
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(device.graphics_index());

    device.create_command_pool(&info)
}
