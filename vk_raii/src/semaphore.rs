use crate::prelude::*;

/// Owns a `VkSemaphore`. The `Device` clone it carries keeps the device alive
/// for as long as the semaphore exists, so `vkDestroySemaphore` can never run
/// after `vkDestroyDevice`.
pub struct Semaphore {
    pub(crate) raw: vk::Semaphore,
    pub(crate) device: Device,
}

impl Semaphore {
    #[inline]
    pub fn handle(&self) -> vk::Semaphore {
        self.raw
    }

    /// The device this semaphore was created from.
    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe { self.device.raw().destroy_semaphore(self.raw, None) };
    }
}
