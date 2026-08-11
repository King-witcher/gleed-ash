use std::fmt;
use std::ops::Deref;

use ash::khr;
use ash::vk;

use super::Instance;

/// Owns a `VkSurfaceKHR` and the `VK_KHR_surface` loader used to query and
/// destroy it. The `Instance` clone keeps the parent alive, so
/// `vkDestroySurfaceKHR` can never run after `vkDestroyInstance`.
///
/// The `vkGetPhysicalDeviceSurface*` queries live on [`PhysicalDevice`], which
/// is where Vulkan puts them; they reach this loader through
/// [`loader`](Self::loader).
///
/// [`PhysicalDevice`]: crate::raii::PhysicalDevice
pub struct Surface {
    raw: vk::SurfaceKHR,
    loader: khr::surface::Instance,
    instance: Instance,
}

impl Surface {
    /// Takes ownership of an already created `VkSurfaceKHR` — the one SDL hands
    /// back, for instance.
    ///
    /// # Safety
    /// - `raw` must have been created from `instance`;
    /// - this takes ownership of `raw`: nothing else may destroy it;
    /// - `VK_KHR_surface` must be enabled on the instance.
    pub unsafe fn from_raw(instance: Instance, raw: vk::SurfaceKHR) -> Self {
        let loader = khr::surface::Instance::new(instance.entry(), &instance);
        Self {
            raw,
            loader,
            instance,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.raw
    }

    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    #[inline]
    pub fn loader(&self) -> &khr::surface::Instance {
        &self.loader
    }
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Surface").field(&self.raw).finish()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_surface(self.raw, None) };
    }
}
