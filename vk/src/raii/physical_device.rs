use ash::prelude::VkResult;
use ash::vk;

use super::{Device, Instance, Surface};

#[derive(Clone)]
pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    instance: Instance,
}

impl PhysicalDevice {
    #[inline]
    pub fn from_handle(instance: Instance, handle: vk::PhysicalDevice) -> Self {
        Self { handle, instance }
    }

    #[inline]
    pub fn handle(&self) -> vk::PhysicalDevice {
        self.handle
    }

    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    #[inline]
    pub unsafe fn properties(&self) -> vk::PhysicalDeviceProperties {
        unsafe {
            self.instance
                .handle()
                .get_physical_device_properties(self.handle)
        }
    }

    #[inline]
    pub unsafe fn features(&self) -> vk::PhysicalDeviceFeatures {
        unsafe {
            self.instance
                .handle()
                .get_physical_device_features(self.handle)
        }
    }

    #[inline]
    pub unsafe fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.instance
                .handle()
                .get_physical_device_memory_properties(self.handle)
        }
    }

    #[inline]
    pub unsafe fn queue_family_properties(&self) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.instance
                .handle()
                .get_physical_device_queue_family_properties(self.handle)
        }
    }

    #[inline]
    pub unsafe fn surface_support(
        &self,
        surface: &Surface,
        queue_family_index: u32,
    ) -> VkResult<bool> {
        unsafe {
            surface.loader().get_physical_device_surface_support(
                self.handle,
                queue_family_index,
                surface.handle(),
            )
        }
    }

    #[inline]
    pub unsafe fn surface_capabilities(
        &self,
        surface: &Surface,
    ) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            surface
                .loader()
                .get_physical_device_surface_capabilities(self.handle, surface.handle())
        }
    }

    #[inline]
    pub unsafe fn surface_formats(&self, surface: &Surface) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            surface
                .loader()
                .get_physical_device_surface_formats(self.handle, surface.handle())
        }
    }

    #[inline]
    pub unsafe fn surface_present_modes(
        &self,
        surface: &Surface,
    ) -> VkResult<Vec<vk::PresentModeKHR>> {
        unsafe {
            surface
                .loader()
                .get_physical_device_surface_present_modes(self.handle, surface.handle())
        }
    }

    pub unsafe fn create_device(&self, create_info: &vk::DeviceCreateInfo) -> VkResult<Device> {
        let handle = unsafe {
            self.instance
                .handle()
                .create_device(self.handle, create_info, None)
        }?;
        Ok(unsafe { Device::from_handle(self.clone(), handle) })
    }
}

impl std::fmt::Debug for PhysicalDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PhysicalDevice").field(&self.handle).finish()
    }
}
