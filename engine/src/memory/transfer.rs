//! The staging path: how data on the CPU reaches a device-local buffer.

use bytemuck::Pod;

use super::buffer::{Buffer, DeviceLocal, HostVisible};
use super::Allocator;
use crate::device::Device;
use crate::prelude::*;

pub struct TransferContext {
    allocator: Allocator,
    queue: vk::raii::Queue,
    /// A single command buffer recycled on every upload: the pool is reset
    /// before recording, instead of allocating and freeing one buffer at a
    /// time.
    command_buffer: vk::raii::CommandBuffer,
    pool: vk::raii::CommandPool,
    fence: vk::raii::Fence,
}

impl TransferContext {
    pub fn new(device: Device, allocator: Allocator) -> Result<Self> {
        let queue = device.queue(device.graphics_index());
        let pool = make_command_pool(&device)?;
        let command_buffer = unsafe { pool.allocate_one(vk::CommandBufferLevel::PRIMARY) }
            .context("allocate the transfer command buffer")?;
        // Not signaled: the first wait must actually wait for the GPU. If it
        // started signaled, it would return before the copy finished.
        let fence = device.create_fence(false)?;

        Ok(Self {
            allocator,
            queue,
            command_buffer,
            pool,
            fence,
        })
    }

    /// Records a closure into the context's command buffer, submits it and
    /// BLOCKS until the GPU finishes. Used for uploads, layout transitions,
    /// etc.
    ///
    /// `&mut self` because recording requires exclusive access to the command
    /// buffer — the same rule Vulkan imposes, only compiler-checked.
    pub fn immediate_submit(
        &mut self,
        record: impl FnOnce(&mut vk::raii::CommandBuffer),
    ) -> Result<()> {
        // The fence wait at the end of the previous call already guaranteed
        // the GPU is done with this buffer, and the `&mut self` that nothing
        // else is recording into it.
        unsafe { self.pool.reset(vk::CommandPoolResetFlags::empty()) }
            .context("reset the transfer command pool")?;

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.command_buffer.begin(&begin_info) }
            .context("begin the transfer command buffer")?;

        record(&mut self.command_buffer);

        unsafe { self.command_buffer.end() }.context("end the transfer command buffer")?;

        let cmd_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(self.command_buffer.handle())];
        let submit_info = vk::SubmitInfo2::default().command_buffer_infos(&cmd_infos);

        // The fence is free: the previous call waited on and reset it, and the
        // buffer just came out of `end`.
        unsafe { self.queue.submit2(&[submit_info], self.fence.handle()) }
            .context("submit the transfer command buffer")?;

        // Blocks until the GPU finishes and leaves the fence ready for reuse.
        unsafe { self.fence.wait(u64::MAX) }.context("wait for the transfer to finish")?;
        unsafe { self.fence.reset() }.context("reset the transfer fence")
    }

    /// Allocates a device-local (VRAM) buffer and uploads `data` into it via
    /// staging. The host-visible staging buffer is created and freed
    /// internally.
    pub fn upload_buffer(
        &mut self,
        data: &[u8],
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer<DeviceLocal>> {
        // 1. Host-visible staging: the CPU writes the data here.
        let staging_info = vk::BufferCreateInfo::default()
            .size(data.len() as u64)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let mut staging = self.allocator.allocate::<HostVisible>(&staging_info)?;
        staging.map_copy(data);

        // 2. Device-local destination (VRAM), also a transfer target.
        let gpu_info = vk::BufferCreateInfo::default()
            .size(data.len() as u64)
            .usage(usage | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let gpu = self.allocator.allocate::<DeviceLocal>(&gpu_info)?;

        // 3. Copies staging -> device-local on the GPU and waits for it.
        self.immediate_submit(|command_buffer| {
            let region = vk::BufferCopy::default().size(data.len() as u64);
            // The buffer is recording (immediate_submit just called begin) and
            // both buffers stay alive until the end of this function.
            unsafe { command_buffer.copy_buffer(staging.vk_buffer(), gpu.vk_buffer(), &[region]) };
        })?;

        // `staging` is dropped here — safe: immediate_submit waited on the
        // fence, so the copy has finished.
        Ok(gpu)
    }

    pub fn upload_slice<T: Pod>(
        &mut self,
        values: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer<DeviceLocal>> {
        self.upload_buffer(bytemuck::cast_slice(values), usage)
    }
}

fn make_command_pool(device: &Device) -> Result<vk::raii::CommandPool> {
    let info = vk::CommandPoolCreateInfo::default()
        // TRANSIENT: the recorded contents are ephemeral — one upload and the
        // pool is reset.
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(device.graphics_index());

    unsafe { device.vk().create_command_pool(&info) }.context("create the command pool")
}
