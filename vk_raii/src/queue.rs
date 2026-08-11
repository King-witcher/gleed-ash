use std::fmt;
use std::ops::Deref;

use ash::prelude::VkResult;
use ash::vk;

use crate::Device;

/// A queue owned by the device. Nothing is created or destroyed here, but the
/// `Device` clone is still needed: the `vkQueue*` entry points come from the
/// device dispatch table.
#[derive(Clone)]
pub struct Queue {
    raw: vk::Queue,
    family_index: u32,
    device: Device,
}

impl Queue {
    /// # Safety
    /// `raw` must have been retrieved from `device` for `family_index`.
    #[inline]
    pub unsafe fn from_raw(device: Device, raw: vk::Queue, family_index: u32) -> Self {
        Self {
            raw,
            family_index,
            device,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Queue {
        self.raw
    }

    #[inline]
    pub fn family_index(&self) -> u32 {
        self.family_index
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// # Safety
    /// - the queue must be externally synchronized;
    /// - every command buffer in `submits` must be in the executable state and
    ///   not already pending;
    /// - each waited semaphore must be signaled or have a signal already
    ///   submitted;
    /// - `fence` must be unsignaled and not in use by another pending submit.
    #[inline]
    pub unsafe fn submit2(&self, submits: &[vk::SubmitInfo2], fence: vk::Fence) -> VkResult<()> {
        unsafe { self.device.queue_submit2(self.raw, submits, fence) }
    }

    /// Blocks until every submission on this queue has finished.
    ///
    /// # Safety
    /// Same contract as `vkQueueWaitIdle`.
    #[inline]
    pub unsafe fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.device.queue_wait_idle(self.raw) }
    }
}

impl Deref for Queue {
    type Target = vk::Queue;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl fmt::Debug for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Queue")
            .field("handle", &self.raw)
            .field("family_index", &self.family_index)
            .finish()
    }
}
