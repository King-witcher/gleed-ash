use ash::{prelude::VkResult, vk};

device_object!(
    DescriptorPool,
    vk::DescriptorPool,
    destroy_descriptor_pool,
    create_descriptor_pool(vk::DescriptorPoolCreateInfo<'_>)
);

impl DescriptorPool {
    pub unsafe fn allocate_sets(
        &self,
        set_layouts: &[vk::DescriptorSetLayout],
    ) -> VkResult<Vec<vk::DescriptorSet>> {
        unsafe {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .set_layouts(set_layouts)
                .descriptor_pool(self.handle);
            self.device.handle().allocate_descriptor_sets(&alloc_info)
        }
    }
}
