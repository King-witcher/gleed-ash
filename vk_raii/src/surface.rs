use crate::prelude::*;

pub struct Surface {
    pub(crate) surface: vk::SurfaceKHR,
    pub(crate) loader: ash::khr::surface::Instance,
    pub(crate) instance: Instance,
}

impl Surface {
    /// Assume a posse de uma `VkSurfaceKHR` já criada (ex.: pela SDL).
    ///
    /// # Safety
    /// - `surface` must have been created from `instance`.
    /// - `Surface` owns the handle: no other code can destroy it.
    /// - `VK_KHR_surface` needs to be enabled in the instance.
    pub unsafe fn from_handle(surface: vk::SurfaceKHR, instance: Instance) -> Surface {
        let loader = ash::khr::surface::Instance::new(instance.entry(), instance.raw());
        Self {
            surface,
            loader,
            instance,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// The instance this surface was created from.
    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}
