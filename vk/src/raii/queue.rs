use ash::prelude::VkResult;
use ash::vk;
use std::fmt;

use super::Device;

#[derive(Clone)]
pub struct Queue {
    handle: vk::Queue,
    family_index: u32,
    device: Device,
}

impl Queue {
    #[inline]
    pub fn from_handle(device: Device, handle: vk::Queue, family_index: u32) -> Self {
        Self {
            handle,
            family_index,
            device,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Queue {
        self.handle
    }

    #[inline]
    pub fn family_index(&self) -> u32 {
        self.family_index
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    #[inline]
    pub unsafe fn submit2(&self, submits: &[vk::SubmitInfo2], fence: vk::Fence) -> VkResult<()> {
        unsafe {
            self.device
                .handle()
                .queue_submit2(self.handle, submits, fence)
        }
    }

    #[inline]
    pub unsafe fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.device.handle().queue_wait_idle(self.handle) }
    }
}

impl fmt::Debug for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Queue")
            .field("handle", &self.handle)
            .field("family_index", &self.family_index)
            .finish()
    }
}
