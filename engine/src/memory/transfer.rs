//! The staging path: how data on the CPU reaches a device-local buffer.

use bytemuck::Pod;
use resource_manager::ImageResource;

use super::buffer::AllocatedBuffer;
use super::Allocator;
use crate::device::Device;
use crate::internal_prelude::*;
use crate::memory::{AllocatedImage, DeviceLocal, HostVisible};

pub struct TransferContext {
    allocator: Allocator,
    queue: vk::raii::Queue,
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
    fn immediate_submit(
        &mut self,
        record: impl FnOnce(&mut vk::raii::CommandBuffer),
    ) -> Result<()> {
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

    pub fn upload_buffer(
        &mut self,
        data: &[u8],
        usage: vk::BufferUsageFlags,
    ) -> Result<AllocatedBuffer<DeviceLocal>> {
        // 1. Host-visible staging: the CPU writes the data here.
        let staging_info = vk::BufferCreateInfo::default()
            .size(data.len() as u64)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let mut staging = self.allocator.create_buffer::<HostVisible>(&staging_info)?;
        staging.map_copy(data);

        // 2. Device-local destination (VRAM), also a transfer target.
        let gpu_info = vk::BufferCreateInfo::default()
            .size(data.len() as u64)
            .usage(usage | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let gpu = self.allocator.create_buffer::<DeviceLocal>(&gpu_info)?;

        // 3. Copies staging -> device-local on the GPU and waits for it.
        self.immediate_submit(|command_buffer| {
            let region = vk::BufferCopy::default().size(data.len() as u64);
            unsafe {
                command_buffer.copy_buffer(
                    staging.vk_buffer().handle(),
                    gpu.vk_buffer().handle(),
                    &[region],
                )
            };
        })?;

        Ok(gpu)
    }

    pub fn upload_image(&mut self, image: &ImageResource) -> Result<AllocatedImage> {
        let extent = vk::Extent3D {
            width: image.width(),
            height: image.height(),
            depth: 1,
        };

        let mut staging = self.allocator.create_buffer::<HostVisible>(
            &vk::BufferCreateInfo::default()
                .size(image.size() as u64)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
        )?;
        staging.map_copy(&image.buffer);

        let allocated_image = self.allocator.create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_SRGB) // Todo: unhardcode it
                .extent(extent)
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
        )?;

        let image_handle = allocated_image.vk_image().handle();

        self.immediate_submit(|command_buffer| {
            unsafe {
                // Maybe different because of barrier2
                image_barrier(
                    command_buffer,
                    image_handle,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::AccessFlags2::NONE,
                    vk::PipelineStageFlags2::NONE,
                    vk::AccessFlags2::TRANSFER_WRITE,
                    vk::PipelineStageFlags2::COPY,
                );

                let copy_region = vk::BufferImageCopy::default()
                    .buffer_offset(0)
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_subresource(image_subresource_layers())
                    .image_extent(extent);

                // test copy buffer to image 2
                command_buffer.copy_buffer_to_image(
                    staging.vk_buffer().handle(),
                    image_handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[copy_region],
                );

                image_barrier(
                    command_buffer,
                    image_handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    vk::AccessFlags2::TRANSFER_WRITE,
                    vk::PipelineStageFlags2::COPY,
                    vk::AccessFlags2::SHADER_READ,
                    vk::PipelineStageFlags2::FRAGMENT_SHADER,
                );
            };
        })?;

        Ok(allocated_image)
    }

    pub fn upload_slice<T: Pod>(
        &mut self,
        values: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<AllocatedBuffer<DeviceLocal>> {
        self.upload_buffer(bytemuck::cast_slice(values), usage)
    }
}

fn make_command_pool(device: &Device) -> Result<vk::raii::CommandPool> {
    let info = vk::CommandPoolCreateInfo::default()
        // TRANSIENT: the recorded contents are ephemeral — one upload and the
        // pool is reset.
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(device.graphics_index());

    unsafe { device.vk_device().create_command_pool(&info) }.context("create the command pool")
}

unsafe fn image_barrier<'a>(
    command_buffer: &mut vk::raii::CommandBuffer,
    image: vk::Image,
    from: vk::ImageLayout,
    to: vk::ImageLayout,
    src_access: vk::AccessFlags2,
    src_stage: vk::PipelineStageFlags2,
    dst_access: vk::AccessFlags2,
    dst_stage: vk::PipelineStageFlags2,
) {
    let barrier = [vk::ImageMemoryBarrier2::default()
        .image(image)
        // src
        .src_access_mask(src_access)
        .src_stage_mask(src_stage)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .old_layout(from)
        // dst
        .dst_access_mask(dst_access)
        .dst_stage_mask(dst_stage)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .new_layout(to)
        .subresource_range(image_subresource_range())];

    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&barrier);

    unsafe { command_buffer.pipeline_barrier2(&dependency_info) };
}

fn image_subresource_layers() -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
    }
}

fn image_subresource_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}
