use crate::prelude::*;

/// Represents a VkPhysicalDevice handle that was valid when it was wrapped.
///
/// Enumerated from a VkInstance and destroyed with it. The `Instance` it carries
/// keeps that instance alive for as long as this handle exists.
///
/// Vulkan requires a physical device and a surface to share a parent instance,
/// so the `surface_*` queries panic when given a `Surface` from another one.
///
/// Cloning is cheap: a raw handle plus a refcount bump on the instance.
#[derive(Clone)]
pub struct PhysicalDevice {
    pub(crate) raw: vk::PhysicalDevice,
    pub(crate) instance: Instance,
}

impl PhysicalDevice {
    #[inline]
    pub fn raw(&self) -> vk::PhysicalDevice {
        self.raw
    }

    /// The instance this physical device was enumerated from.
    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    #[inline]
    pub fn properties(&self) -> vk::PhysicalDeviceProperties {
        unsafe { self.instance.raw().get_physical_device_properties(self.raw) }
    }

    #[inline]
    pub fn features(&self) -> vk::PhysicalDeviceFeatures {
        unsafe { self.instance.raw().get_physical_device_features(self.raw) }
    }

    #[inline]
    pub fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.instance
                .raw()
                .get_physical_device_memory_properties(self.raw)
        }
    }

    #[inline]
    pub fn queue_family_properties(&self) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.instance
                .raw()
                .get_physical_device_queue_family_properties(self.raw)
        }
    }

    #[inline]
    fn assert_same_instance(&self, surface: &Surface) {
        assert!(
            self.instance == surface.instance,
            "surface was created from a different instance than this physical device"
        );
    }

    /// Can this queue family present to `surface`?
    #[inline]
    pub fn surface_support(&self, surface: &Surface, queue_family_index: u32) -> Result<bool> {
        self.assert_same_instance(surface);
        unsafe {
            surface.loader.get_physical_device_surface_support(
                self.raw,
                queue_family_index,
                surface.surface,
            )
        }
    }

    #[inline]
    pub fn surface_capabilities(&self, surface: &Surface) -> Result<vk::SurfaceCapabilitiesKHR> {
        self.assert_same_instance(surface);
        unsafe {
            surface
                .loader
                .get_physical_device_surface_capabilities(self.raw, surface.surface)
        }
    }

    #[inline]
    pub fn surface_formats(&self, surface: &Surface) -> Result<Vec<vk::SurfaceFormatKHR>> {
        self.assert_same_instance(surface);
        unsafe {
            surface
                .loader
                .get_physical_device_surface_formats(self.raw, surface.surface)
        }
    }

    #[inline]
    pub fn surface_present_modes(&self, surface: &Surface) -> Result<Vec<vk::PresentModeKHR>> {
        self.assert_same_instance(surface);
        unsafe {
            surface
                .loader
                .get_physical_device_surface_present_modes(self.raw, surface.surface)
        }
    }

    pub fn create_device(&self, create_info: &vk::DeviceCreateInfo) -> Result<Device> {
        let raw = unsafe {
            self.instance
                .raw()
                .create_device(self.raw, create_info, None)
        }?;

        let device = unsafe { Device::from_raw(self.clone(), raw) };
        Ok(device)
    }
}
