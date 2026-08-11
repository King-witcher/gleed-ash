use std::ops::Deref;

use ash::prelude::VkResult;
use ash::vk;

use crate::{Device, Instance, Surface};

/// A `VkPhysicalDevice` plus the instance that enumerated it.
///
/// Physical devices are never created or destroyed, so this is not an owning
/// wrapper — but a bare `VkPhysicalDevice` is just an opaque integer, and every
/// query on it dispatches through the instance. Carrying the `Instance` is what
/// lets those queries be methods here instead of free functions taking both.
///
/// Cloning is cheap: a raw handle plus a refcount bump on the instance.
#[derive(Clone)]
pub struct PhysicalDevice {
    raw: vk::PhysicalDevice,
    instance: Instance,
}

impl PhysicalDevice {
    /// # Safety
    /// `raw` must have been enumerated from `instance`.
    #[inline]
    pub unsafe fn from_raw(instance: Instance, raw: vk::PhysicalDevice) -> Self {
        Self { raw, instance }
    }

    #[inline]
    pub fn handle(&self) -> vk::PhysicalDevice {
        self.raw
    }

    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// # Safety
    /// Same contract as the `ash` call it forwards to.
    #[inline]
    pub unsafe fn properties(&self) -> vk::PhysicalDeviceProperties {
        unsafe { self.instance.get_physical_device_properties(self.raw) }
    }

    /// # Safety
    /// Same contract as the `ash` call it forwards to.
    #[inline]
    pub unsafe fn features(&self) -> vk::PhysicalDeviceFeatures {
        unsafe { self.instance.get_physical_device_features(self.raw) }
    }

    /// # Safety
    /// Same contract as the `ash` call it forwards to.
    #[inline]
    pub unsafe fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.instance
                .get_physical_device_memory_properties(self.raw)
        }
    }

    /// # Safety
    /// Same contract as the `ash` call it forwards to.
    #[inline]
    pub unsafe fn queue_family_properties(&self) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.instance
                .get_physical_device_queue_family_properties(self.raw)
        }
    }

    /// Can this queue family present to `surface`?
    ///
    /// # Safety
    /// `surface` must come from the same instance as this physical device.
    #[inline]
    pub unsafe fn surface_support(
        &self,
        surface: &Surface,
        queue_family_index: u32,
    ) -> VkResult<bool> {
        unsafe {
            surface.loader().get_physical_device_surface_support(
                self.raw,
                queue_family_index,
                surface.handle(),
            )
        }
    }

    /// # Safety
    /// `surface` must come from the same instance as this physical device.
    #[inline]
    pub unsafe fn surface_capabilities(
        &self,
        surface: &Surface,
    ) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            surface
                .loader()
                .get_physical_device_surface_capabilities(self.raw, surface.handle())
        }
    }

    /// # Safety
    /// `surface` must come from the same instance as this physical device.
    #[inline]
    pub unsafe fn surface_formats(&self, surface: &Surface) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            surface
                .loader()
                .get_physical_device_surface_formats(self.raw, surface.handle())
        }
    }

    /// # Safety
    /// `surface` must come from the same instance as this physical device.
    #[inline]
    pub unsafe fn surface_present_modes(
        &self,
        surface: &Surface,
    ) -> VkResult<Vec<vk::PresentModeKHR>> {
        unsafe {
            surface
                .loader()
                .get_physical_device_surface_present_modes(self.raw, surface.handle())
        }
    }

    /// # Safety
    /// Same contract as `vkCreateDevice`.
    pub unsafe fn create_device(&self, create_info: &vk::DeviceCreateInfo) -> VkResult<Device> {
        let raw = unsafe { self.instance.create_device(self.raw, create_info, None) }?;
        Ok(unsafe { Device::from_raw(self.clone(), raw) })
    }
}

impl Deref for PhysicalDevice {
    type Target = vk::PhysicalDevice;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl std::fmt::Debug for PhysicalDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PhysicalDevice").field(&self.raw).finish()
    }
}
