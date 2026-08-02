use crate::prelude::*;

/// Owns a `VkFence`. The `Device` clone it carries keeps the device alive for
/// as long as the fence exists, so `vkDestroyFence` can never run after
/// `vkDestroyDevice`.
pub struct Fence {
    pub(crate) raw: vk::Fence,
    pub(crate) device: Device,
}

impl Fence {
    #[inline]
    pub fn handle(&self) -> vk::Fence {
        self.raw
    }

    /// The device this fence was created from.
    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Blocks until the fence is signaled. There is no timeout on purpose: a
    /// fence that never signals means the submission that should have signaled
    /// it is missing, which is a bug rather than a condition to recover from.
    pub fn wait(&self) -> Result<()> {
        unsafe {
            self.device
                .raw()
                .wait_for_fences(&[self.raw], true, u64::MAX)
        }
    }

    pub fn reset(&self) -> Result<()> {
        unsafe { self.device.raw().reset_fences(&[self.raw]) }
    }

    /// Waits and leaves the fence unsignaled, ready for the next submit.
    pub fn wait_and_reset(&self) -> Result<()> {
        self.wait()?;
        self.reset()
    }

    /// `true` when the fence is already signaled, without blocking.
    pub fn status(&self) -> Result<bool> {
        unsafe { self.device.raw().get_fence_status(self.raw) }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe { self.device.raw().destroy_fence(self.raw, None) };
    }
}
