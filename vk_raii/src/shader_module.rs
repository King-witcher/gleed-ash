use crate::prelude::*;

/// Owns a `VkShaderModule`. It only needs to outlive the pipeline creation that
/// consumes it, so dropping it right after `vkCreateGraphicsPipelines` is the
/// expected use.
pub struct ShaderModule {
    pub(crate) raw: vk::ShaderModule,
    pub(crate) device: Device,
}

impl ShaderModule {
    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.raw
    }

    /// The device this shader module was created from.
    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe { self.device.raw().destroy_shader_module(self.raw, None) };
    }
}
