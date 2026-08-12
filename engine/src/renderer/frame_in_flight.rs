use super::uniform::UniformBufferObject;
use super::MAX_FRAMES_IN_FLIGHT;
use crate::allocator::{AllocMode, Allocator, Buffer};
use crate::device::Device;
use crate::prelude::*;

pub(super) struct FrameInFlight {
    pub(super) ubo: Buffer,
    /// Um command pool por frame in flight: resetar o pool inteiro por frame é
    /// mais barato do que resetar buffer a buffer. O buffer carrega um clone do
    /// pool, então guardar os dois aqui não impõe ordem nenhuma entre eles.
    pub(super) command_buffer: vk::raii::CommandBuffer,
    pub(super) command_pool: vk::raii::CommandPool,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) image_available: vk::raii::Semaphore,
    pub(super) fence: vk::raii::Fence,
}

impl FrameInFlight {
    pub(super) fn new(
        device: &Device,
        allocator: &Allocator,
        descriptor_pool: &vk::raii::DescriptorPool,
        layout: vk::DescriptorSetLayout,
    ) -> Result<Self> {
        let ubo = make_ubo(allocator)?;
        let command_pool = make_command_pool(device)?;
        let command_buffer = unsafe { command_pool.allocate_one(vk::CommandBufferLevel::PRIMARY) }
            .context("allocate command buffer")?;
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

        // O set acabou de ser alocado: nenhum command buffer pendente o usa.
        unsafe { device.raw().update_descriptor_sets(&writes, &[]) };

        Ok(Self {
            ubo,
            command_buffer,
            command_pool,
            descriptor_set,
            image_available: device.create_semaphore()?,
            fence: device.create_fence(true)?,
        })
    }
}

fn make_command_pool(device: &Device) -> Result<vk::raii::CommandPool> {
    // Sem RESET_COMMAND_BUFFER: o pool inteiro é resetado no início de cada
    // frame, que é o caminho mais barato e dispensa a flag.
    let info = vk::CommandPoolCreateInfo::default().queue_family_index(device.graphics_index());

    device.create_command_pool(&info)
}

pub(super) fn make_descriptor_pool(device: &Device) -> Result<vk::raii::DescriptorPool> {
    // Um descriptor de uniform buffer por frame in flight.
    let pool_sizes = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32)];

    let info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    unsafe { device.vk().create_descriptor_pool(&info) }.context("create descriptor pool")
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
    pool: &vk::raii::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> Result<vk::DescriptorSet> {
    let set_layouts = [layout];
    let info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(pool.handle())
        .set_layouts(&set_layouts);

    // O pool é usado só daqui, na construção dos frames in flight.
    let sets =
        unsafe { device.raw().allocate_descriptor_sets(&info) }.context("allocate descriptor set")?;

    Ok(sets[0])
}
