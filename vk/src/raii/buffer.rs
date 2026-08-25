use ash::{prelude::VkResult, vk};

device_object!(
    Buffer,
    vk::Buffer,
    destroy_buffer,
    create_buffer(vk::BufferCreateInfo<'_>)
);

impl Buffer {
    pub fn memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.device
                .handle()
                .get_buffer_memory_requirements(self.handle.clone())
        }
    }

    pub fn memory_requirements2(&self) -> vk::MemoryRequirements2<'_> {
        unimplemented!()
    }

    pub unsafe fn bind_memory(
        &self,
        memory: vk::DeviceMemory,
        memory_offset: vk::DeviceSize,
    ) -> VkResult<()> {
        unsafe {
            self.device
                .handle()
                .bind_buffer_memory(self.handle, memory, memory_offset)
        }
    }
}
