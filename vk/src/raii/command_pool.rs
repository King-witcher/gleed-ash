use std::fmt;
use std::rc::Rc;

use ash::prelude::VkResult;
use ash::vk;

use super::{CommandBuffer, Device};

#[derive(Clone)]
pub struct CommandPool(Rc<CommandPoolInner>);

struct CommandPoolInner {
    handle: vk::CommandPool,
    device: Device,
}

impl CommandPool {
    #[inline]
    pub unsafe fn from_handle(device: Device, handle: vk::CommandPool) -> Self {
        Self(Rc::new(CommandPoolInner { handle, device }))
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.0.handle
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.0.device
    }

    pub unsafe fn allocate(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> VkResult<Vec<CommandBuffer>> {
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.0.handle)
            .command_buffer_count(count)
            .level(level);

        let handles = unsafe { self.0.device.handle().allocate_command_buffers(&info) }?;
        Ok(handles
            .into_iter()
            .map(|handle| unsafe { CommandBuffer::from_handle(self.clone(), handle) })
            .collect())
    }

    pub unsafe fn allocate_one(&self, level: vk::CommandBufferLevel) -> VkResult<CommandBuffer> {
        let mut buffers = unsafe { self.allocate(level, 1) }?;
        Ok(buffers.remove(0))
    }

    #[inline]
    pub unsafe fn reset(&self, flags: vk::CommandPoolResetFlags) -> VkResult<()> {
        unsafe {
            self.0
                .device
                .handle()
                .reset_command_pool(self.0.handle, flags)
        }
    }
}

impl fmt::Debug for CommandPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandPool").field(&self.0.handle).finish()
    }
}

impl PartialEq for CommandPool {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for CommandPool {}

impl Drop for CommandPoolInner {
    fn drop(&mut self) {
        unsafe { self.device.handle().destroy_command_pool(self.handle, None) };
    }
}
