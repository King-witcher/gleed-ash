//! Equivalente a modules/engine/src/transfer.{h,cc}.

use std::rc::Rc;

use ash::vk;

use crate::allocator::{as_bytes, AllocMode, Allocator, Buffer};
use crate::device::Device;
use crate::prelude::*;

pub struct TransferContext {
    device: Device,
    allocator: Rc<Allocator>,
    queue: vk_raii::Queue,
    pool: vk::CommandPool,
    /// RAII: carrega um clone do device e se destrói junto com o contexto.
    fence: vk_raii::Fence,
}

impl TransferContext {
    pub fn new(device: Device, allocator: Rc<Allocator>) -> Result<Self> {
        let queue = device.get_queue(device.graphics_index());
        let pool = make_command_pool(&device)?;
        // Não sinalizada: o primeiro wait_for_fence precisa realmente esperar a
        // GPU. Se começasse sinalizada, a espera retornaria antes da cópia terminar.
        let fence = device.create_fence(false)?;

        Ok(Self {
            device,
            allocator,
            queue,
            pool,
            fence,
        })
    }

    /// Grava um closure em um command buffer descartável, submete e BLOQUEIA até
    /// a GPU terminar. Usado para uploads, transições de layout, etc.
    pub fn immediate_submit(&self, record: impl FnOnce(vk::CommandBuffer)) -> Result<()> {
        let raw = self.device.raw();

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        let buffers = unsafe { raw.allocate_command_buffers(&alloc_info) }
            .context("alocar transfer command buffer")?;
        let cmd = buffers[0];

        unsafe {
            raw.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .context("iniciar transfer command buffer")?;

            record(cmd);

            raw.end_command_buffer(cmd)
                .context("finalizar transfer command buffer")?;

            let cmd_infos = [vk::CommandBufferSubmitInfo::default().command_buffer(cmd)];
            let submit_info = vk::SubmitInfo2::default().command_buffer_infos(&cmd_infos);

            self.queue
                .submit2(&[submit_info], Some(&self.fence))
                .context("submeter transfer command buffer")?;
        }

        // Bloqueia até a GPU terminar e deixa a fence pronta para o próximo uso.
        self.fence
            .wait_and_reset()
            .context("esperar a transferência terminar")?;

        // Diferente do C++ (onde o vk::raii::CommandBuffer se devolvia sozinho
        // ao pool), aqui a devolução é explícita.
        unsafe { raw.free_command_buffers(self.pool, &buffers) };

        Ok(())
    }

    /// Aloca um buffer device-local (VRAM) e sobe `data` para ele via staging.
    /// O buffer de staging host-visible é criado e liberado internamente.
    pub fn upload_buffer(&self, data: &[u8], usage: vk::BufferUsageFlags) -> Result<Buffer> {
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
        let raw = self.device.raw();
        self.immediate_submit(|cmd| {
            let region = vk::BufferCopy::default().size(data.len() as u64);
            unsafe { raw.cmd_copy_buffer(cmd, staging.vk_buffer(), gpu.vk_buffer(), &[region]) };
        })?;

        // `staging` é destruído aqui — seguro, já que immediate_submit esperou a
        // fence, garantindo que a cópia terminou.
        Ok(gpu)
    }

    pub fn upload_slice<T: Copy>(
        &self,
        values: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer> {
        self.upload_buffer(as_bytes(values), usage)
    }
}

impl Drop for TransferContext {
    fn drop(&mut self) {
        unsafe { self.device.raw().destroy_command_pool(self.pool, None) };
    }
}

fn make_command_pool(device: &Device) -> Result<vk::CommandPool> {
    let info = vk::CommandPoolCreateInfo::default()
        // TRANSIENT: os command buffers são efêmeros (um upload e são liberados).
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(device.graphics_index());

    unsafe { device.raw().create_command_pool(&info, None) }.context("criar transfer command pool")
}
