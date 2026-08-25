use std::fmt;
use std::rc::Rc;

use ash::prelude::VkResult;
use ash::vk;

use super::PhysicalDevice;

#[derive(Clone)]
pub struct Instance(Rc<InstanceInner>);

struct InstanceInner {
    handle: ash::Instance,
    entry: ash::Entry,
}

impl Instance {
    pub unsafe fn new(entry: ash::Entry, create_info: &vk::InstanceCreateInfo) -> VkResult<Self> {
        let handle = unsafe { entry.create_instance(create_info, None) }?;
        Ok(unsafe { Self::from_handle(entry, handle) })
    }

    #[inline]
    pub unsafe fn from_handle(entry: ash::Entry, handle: ash::Instance) -> Self {
        Self(Rc::new(InstanceInner { handle, entry }))
    }

    #[inline]
    pub fn handle(&self) -> &ash::Instance {
        &self.0.handle
    }

    #[inline]
    pub fn vk_entry(&self) -> &ash::Entry {
        &self.0.entry
    }

    pub unsafe fn enumerate_physical_devices(&self) -> VkResult<Vec<PhysicalDevice>> {
        let handles = unsafe { self.0.handle.enumerate_physical_devices() }?;
        Ok(handles
            .into_iter()
            .map(|handle| PhysicalDevice::from_handle(self.clone(), handle))
            .collect())
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Instance")
            .field(&self.0.handle.handle())
            .finish()
    }
}

impl PartialEq for Instance {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Instance {}

impl Drop for InstanceInner {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_instance(None) };
    }
}
