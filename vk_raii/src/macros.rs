/// Declares a wrapper that owns a device-level handle.
///
/// Generates the struct, `Deref` to the raw handle, `Debug`, and a `Drop` that
/// calls `$destroy`. The optional fourth argument also adds the matching
/// constructor to [`Device`](crate::Device), shadowing the `ash` method of the
/// same name so it returns the wrapper instead of the bare handle.
macro_rules! device_object {
    ($name:ident, $handle:ty, $destroy:ident) => {
        pub struct $name {
            raw: $handle,
            device: $crate::Device,
        }

        impl $name {
            /// # Safety
            /// - `raw` must have been created from `device`;
            /// - this takes ownership of `raw`: nothing else may destroy it.
            #[inline]
            pub unsafe fn from_raw(device: $crate::Device, raw: $handle) -> Self {
                Self { raw, device }
            }

            #[inline]
            pub fn handle(&self) -> $handle {
                self.raw
            }

            #[inline]
            pub fn device(&self) -> &$crate::Device {
                &self.device
            }
        }

        impl ::std::ops::Deref for $name {
            type Target = $handle;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.raw
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.raw).finish()
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { ::ash::Device::$destroy(&self.device, self.raw, None) };
            }
        }
    };

    ($name:ident, $handle:ty, $destroy:ident, $create:ident($info:ty)) => {
        device_object!($name, $handle, $destroy);

        impl $crate::Device {
            /// # Safety
            /// Same contract as the `ash` call it forwards to.
            #[inline]
            pub unsafe fn $create(&self, create_info: &$info) -> ::ash::prelude::VkResult<$name> {
                let raw = unsafe { ::ash::Device::$create(self, create_info, None) }?;
                Ok($name {
                    raw,
                    device: self.clone(),
                })
            }
        }
    };
}
