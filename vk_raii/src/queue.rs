use crate::prelude::*;

/// A queue owned by the device. Nothing is created or destroyed here, but the
/// `Device` clone is still needed: the `vkQueue*` entry points come from the
/// device dispatch table.
#[derive(Clone)]
pub struct Queue {
    pub(crate) raw: vk::Queue,
    pub(crate) queue_family_index: u32,
    pub(crate) device: Device,
}

impl Queue {
    #[inline]
    pub fn handle(&self) -> vk::Queue {
        self.raw
    }

    #[inline]
    pub fn family_index(&self) -> u32 {
        self.queue_family_index
    }

    /// The device this queue belongs to.
    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// # Safety
    /// - every command buffer in `submits` must be in the executable state and
    ///   not already pending;
    /// - each waited semaphore must be signaled or have a signal already
    ///   submitted;
    /// - `fence` must be unsignaled and not in use by another pending submit.
    pub unsafe fn submit2(&self, submits: &[vk::SubmitInfo2], fence: Option<&Fence>) -> Result<()> {
        let fence = fence.map_or(vk::Fence::null(), Fence::handle);
        unsafe { self.device.raw().queue_submit2(self.raw, submits, fence) }
    }

    /// Blocks until every submission on this queue has finished.
    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.device.raw().queue_wait_idle(self.raw) }
    }
}
