use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use ash::prelude::VkResult;
use ash::vk;

use super::{CommandBuffer, Device};

/// Cheap handle to a `VkCommandPool`: cloning bumps a refcount, and the pool is
/// destroyed once the last clone — including the ones held by every
/// [`CommandBuffer`] allocated from it — is gone. Freeing a buffer needs the
/// pool it came from, which is why the refcount is here and not just on the
/// device.
#[derive(Clone)]
pub struct CommandPool(Rc<CommandPoolInner>);

struct CommandPoolInner {
    raw: vk::CommandPool,
    device: Device,
}

impl CommandPool {
    /// # Safety
    /// - `raw` must have been created from `device`;
    /// - this takes ownership of `raw`: nothing else may destroy it.
    #[inline]
    pub unsafe fn from_raw(device: Device, raw: vk::CommandPool) -> Self {
        Self(Rc::new(CommandPoolInner { raw, device }))
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.0.raw
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.0.device
    }

    /// # Safety
    /// The pool must be externally synchronized.
    pub unsafe fn allocate(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> VkResult<Vec<CommandBuffer>> {
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.0.raw)
            .command_buffer_count(count)
            .level(level);

        let raws = unsafe { self.0.device.allocate_command_buffers(&info) }?;
        Ok(raws
            .into_iter()
            .map(|raw| unsafe { CommandBuffer::from_raw(self.clone(), raw) })
            .collect())
    }

    /// # Safety
    /// The pool must be externally synchronized.
    pub unsafe fn allocate_one(&self, level: vk::CommandBufferLevel) -> VkResult<CommandBuffer> {
        let mut buffers = unsafe { self.allocate(level, 1) }?;
        Ok(buffers.remove(0))
    }

    /// Recycles every command buffer of the pool at once, which is cheaper than
    /// resetting them one by one — and lets the pool skip
    /// `RESET_COMMAND_BUFFER`.
    ///
    /// # Safety
    /// - the pool must be externally synchronized;
    /// - none of its command buffers may be pending execution.
    #[inline]
    pub unsafe fn reset(&self, flags: vk::CommandPoolResetFlags) -> VkResult<()> {
        unsafe { self.0.device.reset_command_pool(self.0.raw, flags) }
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.raw
    }
}

impl fmt::Debug for CommandPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandPool").field(&self.0.raw).finish()
    }
}

/// Handle identity: two `CommandPool` values are equal when they refer to the
/// same `VkCommandPool`.
impl PartialEq for CommandPool {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for CommandPool {}

impl Drop for CommandPoolInner {
    fn drop(&mut self) {
        // Destroying the pool also frees any command buffer still allocated
        // from it — but every `CommandBuffer` holds a clone of this handle, so
        // this only runs once they are all gone.
        unsafe { self.device.destroy_command_pool(self.raw, None) };
    }
}
