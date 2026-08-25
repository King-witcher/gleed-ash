use std::fmt;

use ash::khr;
use ash::prelude::VkResult;
use ash::vk;

use super::{Device, Queue};

pub struct Swapchain {
    handle: vk::SwapchainKHR,
    loader: khr::swapchain::Device,
    device: Device,
}

impl Swapchain {
    #[inline]
    pub unsafe fn from_handle(
        device: Device,
        loader: khr::swapchain::Device,
        handle: vk::SwapchainKHR,
    ) -> Self {
        Self {
            handle,
            loader,
            device,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::SwapchainKHR {
        self.handle
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    #[inline]
    pub fn loader(&self) -> &khr::swapchain::Device {
        &self.loader
    }

    #[inline]
    pub unsafe fn images(&self) -> VkResult<Vec<vk::Image>> {
        unsafe { self.loader.get_swapchain_images(self.handle) }
    }

    #[inline]
    pub unsafe fn acquire_next_image(
        &self,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> VkResult<(u32, bool)> {
        unsafe {
            self.loader
                .acquire_next_image(self.handle, timeout, semaphore, fence)
        }
    }

    #[inline]
    pub unsafe fn queue_present(
        &self,
        queue: &Queue,
        present_info: &vk::PresentInfoKHR,
    ) -> VkResult<bool> {
        unsafe { self.loader.queue_present(queue.handle(), present_info) }
    }
}

impl fmt::Debug for Swapchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Swapchain").field(&self.handle).finish()
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_swapchain(self.handle, None) };
    }
}
