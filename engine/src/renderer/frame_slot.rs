use super::uniform::Transforms;
use super::MAX_FRAMES_IN_FLIGHT;
use crate::device::Device;
use crate::internal_prelude::*;
use crate::memory::{AllocatedBuffer, Allocator, HostVisible};

#[derive(Debug)]
pub(super) struct FrameSlot {
    pub(super) transforms_buffer: AllocatedBuffer<HostVisible>,
    pub(super) command_buffer: vk::raii::CommandBuffer, // Each slot is reset all at once
    pub(super) command_pool: vk::raii::CommandPool,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) image_available: vk::raii::Semaphore,
    pub(super) fence: vk::raii::Fence,
}

impl FrameSlot {
    pub(super) fn new(
        device: &Device,
        allocator: &Allocator,
        descriptor_pool: &vk::raii::DescriptorPool,
        layout: vk::DescriptorSetLayout,
    ) -> Result<Self> {
        let ubo = make_ubo(allocator)?;
        let command_pool = make_command_pool(device)?;
        let command_buffer = unsafe { command_pool.allocate_one(vk::CommandBufferLevel::PRIMARY) }
            .context("allocate the command buffer")?;
        let descriptor_set = make_descriptor_set(descriptor_pool, layout)?;

        let buffer_info = [vk::DescriptorBufferInfo::default()
            .buffer(ubo.vk_buffer().handle())
            .offset(0)
            .range(std::mem::size_of::<Transforms>() as u64)];

        let write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_info);

        unsafe { device.vk_device().update_descriptor_sets(&[write], &[]) };

        Ok(Self {
            transforms_buffer: ubo,
            command_buffer,
            command_pool,
            descriptor_set,
            image_available: device.create_semaphore()?,
            fence: device.create_fence(true)?,
        })
    }
}

fn make_command_pool(device: &Device) -> Result<vk::raii::CommandPool> {
    // No RESET_COMMAND_BUFFER: the whole pool is reset at the start of each
    // frame, which is the cheapest path and makes the flag unnecessary.
    let info = vk::CommandPoolCreateInfo::default().queue_family_index(device.graphics_index());

    unsafe { device.vk_device().create_command_pool(&info) }.context("create the command pool")
}

pub(super) fn make_descriptor_pool(device: &Device) -> Result<vk::raii::DescriptorPool> {
    let pool_sizes = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32)];

    let info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    unsafe { device.vk_device().create_descriptor_pool(&info) }
        .context("create the descriptor pool")
}

fn make_ubo(allocator: &Allocator) -> Result<AllocatedBuffer<HostVisible>> {
    let info = vk::BufferCreateInfo::default()
        .size(std::mem::size_of::<Transforms>() as u64)
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    // HostVisible: the CPU rewrites the transforms every frame.
    allocator.create_buffer(&info)
}

fn make_descriptor_set(
    pool: &vk::raii::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> Result<vk::DescriptorSet> {
    let set_layouts = [layout];

    let sets =
        unsafe { pool.allocate_sets(&set_layouts) }.context("allocate the descriptor set")?;

    Ok(sets[0])
}
