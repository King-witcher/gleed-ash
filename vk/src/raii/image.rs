use ash::{prelude::VkResult, vk};

device_object!(
    Image,
    vk::Image,
    destroy_image,
    create_image(vk::ImageCreateInfo<'_>)
);

impl Image {
    #[inline]
    pub unsafe fn memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.device
                .handle()
                .get_image_memory_requirements(self.handle)
        }
    }

    #[inline]
    pub unsafe fn memory_requirements2(
        &self,
        info: &vk::ImageMemoryRequirementsInfo2,
    ) -> vk::MemoryRequirements2<'_> {
        let mut requirements = Default::default();
        unsafe {
            self.device
                .handle()
                .get_image_memory_requirements2(info, &mut requirements);
        }
        requirements
    }

    #[inline]
    pub unsafe fn bind_memory(
        &mut self,
        memory: vk::DeviceMemory,
        memory_offset: vk::DeviceSize,
    ) -> VkResult<()> {
        unsafe {
            self.device
                .handle()
                .bind_image_memory(self.handle, memory, memory_offset)
        }
    }
}
