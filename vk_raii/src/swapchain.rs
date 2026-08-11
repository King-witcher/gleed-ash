use std::fmt;
use std::ops::Deref;

use ash::khr;
use ash::prelude::VkResult;
use ash::vk;

use crate::{Device, Queue};

/// Owns a `VkSwapchainKHR` and the `VK_KHR_swapchain` loader that destroys it.
///
/// Created through [`Device::create_swapchain`]. Note that the image views made
/// from [`images`](Self::images) have to die before this does — Vulkan requires
/// it, and nothing here enforces it.
pub struct Swapchain {
    raw: vk::SwapchainKHR,
    loader: khr::swapchain::Device,
    device: Device,
}

impl Swapchain {
    /// # Safety
    /// - `raw` must have been created from `device` through `loader`;
    /// - this takes ownership of `raw`: nothing else may destroy it.
    #[inline]
    pub unsafe fn from_raw(
        device: Device,
        loader: khr::swapchain::Device,
        raw: vk::SwapchainKHR,
    ) -> Self {
        Self {
            raw,
            loader,
            device,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::SwapchainKHR {
        self.raw
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    #[inline]
    pub fn loader(&self) -> &khr::swapchain::Device {
        &self.loader
    }

    /// The images are owned by the swapchain, so they come back as raw handles:
    /// there is nothing to destroy per image.
    ///
    /// # Safety
    /// Same contract as `vkGetSwapchainImagesKHR`.
    #[inline]
    pub unsafe fn images(&self) -> VkResult<Vec<vk::Image>> {
        unsafe { self.loader.get_swapchain_images(self.raw) }
    }

    /// Returns the image index and whether the swapchain is suboptimal.
    ///
    /// # Safety
    /// Same contract as `vkAcquireNextImageKHR`.
    #[inline]
    pub unsafe fn acquire_next_image(
        &self,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> VkResult<(u32, bool)> {
        unsafe {
            self.loader
                .acquire_next_image(self.raw, timeout, semaphore, fence)
        }
    }

    /// Returns whether the swapchain is suboptimal.
    ///
    /// # Safety
    /// Same contract as `vkQueuePresentKHR`, including the external
    /// synchronization of `queue`.
    #[inline]
    pub unsafe fn queue_present(
        &self,
        queue: &Queue,
        present_info: &vk::PresentInfoKHR,
    ) -> VkResult<bool> {
        unsafe { self.loader.queue_present(queue.handle(), present_info) }
    }
}

impl Deref for Swapchain {
    type Target = vk::SwapchainKHR;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl fmt::Debug for Swapchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Swapchain").field(&self.raw).finish()
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_swapchain(self.raw, None) };
    }
}
