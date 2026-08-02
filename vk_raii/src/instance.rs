use crate::prelude::*;

/// Cheap handle to a `VkInstance`: cloning bumps a refcount, and the instance is
/// destroyed once the last clone — including the ones held by `PhysicalDevice`
/// and `Surface` — is gone.
#[derive(Clone)]
pub struct Instance(Rc<InstanceInner>);

struct InstanceInner {
    raw: ash::Instance,
    entry: ash::Entry,
}

impl Instance {
    /// # Safety
    /// - The pointers reachable from `create_info` must be valid for the call:
    ///   null-terminated strings and arrays matching their declared counts.
    /// - Loads the Vulkan loader from the system, with whatever that entails.
    pub unsafe fn new(create_info: &vk::InstanceCreateInfo) -> Result<Instance> {
        let entry = unsafe { ash::Entry::load() }.expect("failed to load Vulkan loader");
        let raw = unsafe { entry.create_instance(create_info, None) }?;

        Ok(Self(Rc::new(InstanceInner { raw, entry })))
    }

    #[inline]
    pub fn handle(&self) -> vk::Instance {
        self.0.raw.handle()
    }

    #[inline]
    pub fn raw(&self) -> &ash::Instance {
        &self.0.raw
    }

    #[inline]
    pub(crate) fn entry(&self) -> &ash::Entry {
        &self.0.entry
    }

    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let physical_devices = unsafe { self.0.raw.enumerate_physical_devices() }?;
        Ok(physical_devices
            .into_iter()
            .map(|raw| PhysicalDevice {
                raw,
                instance: self.clone(),
            })
            .collect())
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
