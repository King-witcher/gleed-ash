use ash::vk;

device_object!(
    Semaphore,
    vk::Semaphore,
    destroy_semaphore,
    create_semaphore(vk::SemaphoreCreateInfo<'_>)
);

device_object!(
    ShaderModule,
    vk::ShaderModule,
    destroy_shader_module,
    create_shader_module(vk::ShaderModuleCreateInfo<'_>)
);

device_object!(
    ImageView,
    vk::ImageView,
    destroy_image_view,
    create_image_view(vk::ImageViewCreateInfo<'_>)
);

device_object!(
    DescriptorSetLayout,
    vk::DescriptorSetLayout,
    destroy_descriptor_set_layout,
    create_descriptor_set_layout(vk::DescriptorSetLayoutCreateInfo<'_>)
);

device_object!(
    PipelineLayout,
    vk::PipelineLayout,
    destroy_pipeline_layout,
    create_pipeline_layout(vk::PipelineLayoutCreateInfo<'_>)
);

// No single-object create call: pipelines come out of a batch, so
// `Device::create_graphics_pipeline` builds these by hand.
device_object!(Pipeline, vk::Pipeline, destroy_pipeline);
