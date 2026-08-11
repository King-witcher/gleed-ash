//! Device-owned handles whose whole wrapper is the RAII pattern: own the
//! handle, deref to it, destroy it on drop.

use ash::prelude::VkResult;
use ash::vk;

device_object!(
    Semaphore,
    vk::Semaphore,
    destroy_semaphore,
    create_semaphore(vk::SemaphoreCreateInfo<'_>)
);

device_object!(
    Fence,
    vk::Fence,
    destroy_fence,
    create_fence(vk::FenceCreateInfo<'_>)
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
    DescriptorPool,
    vk::DescriptorPool,
    destroy_descriptor_pool,
    create_descriptor_pool(vk::DescriptorPoolCreateInfo<'_>)
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
// `Device::create_graphics_pipelines` builds these by hand.
device_object!(Pipeline, vk::Pipeline, destroy_pipeline);

impl Fence {
    /// # Safety
    /// Same contract as `vkWaitForFences`.
    #[inline]
    pub unsafe fn wait(&self, timeout: u64) -> VkResult<()> {
        unsafe {
            self.device()
                .wait_for_fences(&[self.handle()], true, timeout)
        }
    }

    /// # Safety
    /// Same contract as `vkResetFences` — in particular, the fence must not be
    /// in use by a pending submit.
    #[inline]
    pub unsafe fn reset(&self) -> VkResult<()> {
        unsafe { self.device().reset_fences(&[self.handle()]) }
    }

    /// `true` when the fence is already signaled, without blocking.
    ///
    /// # Safety
    /// Same contract as `vkGetFenceStatus`.
    #[inline]
    pub unsafe fn status(&self) -> VkResult<bool> {
        unsafe { self.device().get_fence_status(self.handle()) }
    }
}
