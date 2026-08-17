use super::uniform::Transforms;
use super::MAX_FRAMES_IN_FLIGHT;
use crate::device::Device;
use crate::internal_prelude::*;
use crate::memory::{Allocator, Buffer, HostVisible};

/// The persistent resources of one frame in flight, recycled round-robin by
/// the [`super::Renderer`]. A [`super::Frame`] borrows from the slot it lands
/// on for as long as it is being recorded.
pub(super) struct FrameSlot {
    pub(super) transforms_buffer: Buffer<HostVisible>,
    /// One command pool per slot: resetting the whole pool per frame is
    /// cheaper than resetting buffer by buffer. The buffer carries a clone of
    /// the pool, so holding both here imposes no ordering between them.
    pub(super) command_buffer: vk::raii::CommandBuffer,
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
        let descriptor_set = make_descriptor_set(device, descriptor_pool, layout)?;

        // Points this slot's descriptor set at its own UBO. The buffer is
        // persistent, so this binding is written once and only the contents
        // change per frame — no need to rewrite the descriptor set every frame.
        let buffer_infos = [vk::DescriptorBufferInfo::default()
            .buffer(ubo.vk_buffer())
            .offset(0)
            .range(std::mem::size_of::<Transforms>() as u64)];

        let writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_infos)];

        // The set was just allocated: no pending command buffer uses it.
        unsafe { device.raw().update_descriptor_sets(&writes, &[]) };

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

    unsafe { device.vk().create_command_pool(&info) }.context("create the command pool")
}

pub(super) fn make_descriptor_pool(device: &Device) -> Result<vk::raii::DescriptorPool> {
    // One uniform buffer descriptor per slot.
    let pool_sizes = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32)];

    let info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    unsafe { device.vk().create_descriptor_pool(&info) }.context("create the descriptor pool")
}

fn make_ubo(allocator: &Allocator) -> Result<Buffer<HostVisible>> {
    let info = vk::BufferCreateInfo::default()
        .size(std::mem::size_of::<Transforms>() as u64)
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    // HostVisible: the CPU rewrites the transforms every frame.
    allocator.allocate(&info)
}

fn make_descriptor_set(
    device: &Device,
    pool: &vk::raii::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> Result<vk::DescriptorSet> {
    let set_layouts = [layout];
    let info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(pool.handle())
        .set_layouts(&set_layouts);

    // The pool is only used from here, while building the slots.
    let sets = unsafe { device.raw().allocate_descriptor_sets(&info) }
        .context("allocate the descriptor set")?;

    Ok(sets[0])
}
