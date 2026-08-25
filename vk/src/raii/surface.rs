use std::fmt;

use ash::khr;
use ash::vk;

use super::Instance;

pub struct Surface {
    handle: vk::SurfaceKHR,
    surface_loader: khr::surface::Instance,
    instance: Instance,
}

impl Surface {
    pub unsafe fn from_handle(instance: Instance, handle: vk::SurfaceKHR) -> Self {
        let surface_loader = khr::surface::Instance::new(instance.vk_entry(), instance.handle());
        Self {
            handle,
            surface_loader,
            instance,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.handle
    }

    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    #[inline]
    pub fn loader(&self) -> &khr::surface::Instance {
        &self.surface_loader
    }
}

impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Surface").field(&self.handle).finish()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.surface_loader.destroy_surface(self.handle, None) };
    }
}
