use ash::{prelude::*, vk};

device_object!(
    Fence,
    vk::Fence,
    destroy_fence,
    create_fence(vk::FenceCreateInfo<'_>)
);

impl Fence {
    #[inline]
    pub unsafe fn wait(&self, timeout: u64) -> VkResult<()> {
        unsafe {
            self.device()
                .handle()
                .wait_for_fences(&[self.handle()], true, timeout)
        }
    }

    #[inline]
    pub unsafe fn reset(&self) -> VkResult<()> {
        unsafe { self.device().handle().reset_fences(&[self.handle()]) }
    }

    #[inline]
    pub unsafe fn status(&self) -> VkResult<bool> {
        unsafe { self.device().handle().get_fence_status(self.handle()) }
    }
}
