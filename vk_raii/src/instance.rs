use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use ash::prelude::VkResult;
use ash::vk;

use crate::PhysicalDevice;

/// Cheap handle to a `VkInstance`: cloning bumps a refcount, and the instance is
/// destroyed once the last clone — including the ones held by `PhysicalDevice`
/// and `Surface` — is gone.
///
/// Derefs to [`ash::Instance`], so every `ash` entry point is available; the
/// inherent methods below shadow the ones that should hand back a wrapper.
#[derive(Clone)]
pub struct Instance(Rc<InstanceInner>);

struct InstanceInner {
    raw: ash::Instance,
    /// The loader owns the shared library the instance dispatch table points
    /// into, so it has to outlive the instance.
    entry: ash::Entry,
}

impl Instance {
    /// # Safety
    /// - the pointers reachable from `create_info` must be valid for the call:
    ///   null-terminated strings and arrays matching their declared counts.
    pub unsafe fn new(entry: ash::Entry, create_info: &vk::InstanceCreateInfo) -> VkResult<Self> {
        let raw = unsafe { entry.create_instance(create_info, None) }?;
        Ok(unsafe { Self::from_raw(entry, raw) })
    }

    /// # Safety
    /// - `raw` must have been created from `entry`;
    /// - this takes ownership of `raw`: nothing else may destroy it.
    #[inline]
    pub unsafe fn from_raw(entry: ash::Entry, raw: ash::Instance) -> Self {
        Self(Rc::new(InstanceInner { raw, entry }))
    }

    #[inline]
    pub fn entry(&self) -> &ash::Entry {
        &self.0.entry
    }

    /// # Safety
    /// Same contract as `vkEnumeratePhysicalDevices`.
    pub unsafe fn enumerate_physical_devices(&self) -> VkResult<Vec<PhysicalDevice>> {
        let raws = unsafe { self.0.raw.enumerate_physical_devices() }?;
        Ok(raws
            .into_iter()
            .map(|raw| unsafe { PhysicalDevice::from_raw(self.clone(), raw) })
            .collect())
    }
}

impl Deref for Instance {
    type Target = ash::Instance;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.raw
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Instance")
            .field(&self.0.raw.handle())
            .finish()
    }
}

/// Handle identity: two `Instance` values are equal when they refer to the same
/// `VkInstance`.
impl PartialEq for Instance {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Instance {}

impl Drop for InstanceInner {
    fn drop(&mut self) {
        unsafe { self.raw.destroy_instance(None) };
    }
}
