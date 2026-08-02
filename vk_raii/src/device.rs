use crate::prelude::*;

/// Cheap handle to a `VkDevice`: cloning bumps a refcount, and the device is
/// destroyed once the last clone — including the ones held by every object it
/// created — is gone.
#[derive(Clone)]
pub struct Device(Rc<DeviceInner>);

struct DeviceInner {
    raw: ash::Device,
    /// Keeps the physical device, and through it the instance, alive: the
    /// device has to be destroyed before `vkDestroyInstance`.
    physical_device: PhysicalDevice,
}

impl Device {
    /// # Safety
    /// - `raw` must have been created from `physical_device`.
    /// - `Device` owns the handle: no other code can destroy it.
    #[inline]
    pub unsafe fn from_raw(physical_device: PhysicalDevice, raw: ash::Device) -> Self {
        Self(Rc::new(DeviceInner {
            raw,
            physical_device,
        }))
    }

    #[inline]
    pub fn handle(&self) -> vk::Device {
        self.0.raw.handle()
    }

    #[inline]
    pub fn raw(&self) -> &ash::Device {
        &self.0.raw
    }

    /// The physical device this device was created from.
    #[inline]
    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.0.physical_device
    }

    /// The instance this device was created from.
    #[inline]
    pub fn instance(&self) -> &Instance {
        self.0.physical_device.instance()
    }

    /// Looks up a queue the device already owns — nothing is created here, so
    /// there is nothing to destroy either. The family must be one of those
    /// asked for in the `VkDeviceCreateInfo`, and `queue_index` must be below
    /// the count requested for it.
    #[inline]
    pub fn queue(&self, queue_family_index: u32, queue_index: u32) -> Queue {
        let raw = unsafe { self.0.raw.get_device_queue(queue_family_index, queue_index) };
        Queue {
            raw,
            queue_family_index,
            device: self.clone(),
        }
    }

    pub fn create_semaphore(&self, create_info: &vk::SemaphoreCreateInfo) -> Result<Semaphore> {
        let raw = unsafe { self.0.raw.create_semaphore(create_info, None) }?;
        Ok(Semaphore {
            raw,
            device: self.clone(),
        })
    }

    pub fn create_fence(&self, create_info: &vk::FenceCreateInfo) -> Result<Fence> {
        let raw = unsafe { self.0.raw.create_fence(create_info, None) }?;
        Ok(Fence {
            raw,
            device: self.clone(),
        })
    }

    pub fn create_shader_module(
        &self,
        create_info: &vk::ShaderModuleCreateInfo,
    ) -> Result<ShaderModule> {
        let raw = unsafe { self.0.raw.create_shader_module(create_info, None) }?;
        Ok(ShaderModule {
            raw,
            device: self.clone(),
        })
    }

    /// Blocks until every queue of this device is idle.
    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.0.raw.device_wait_idle() }
    }
}

/// Handle identity: two `Device` values are equal when they refer to the same
/// `VkDevice`.
impl PartialEq for Device {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Device {}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe { self.raw.destroy_device(None) };
    }
}
