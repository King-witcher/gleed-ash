macro_rules! device_object {
    ($name:ident, $handle:ty, $destroy:ident) => {
        pub struct $name {
            handle: $handle,
            device: $crate::raii::Device,
        }

        impl $name {
            #[inline]
            pub unsafe fn from_handle(device: $crate::raii::Device, handle: $handle) -> Self {
                Self { handle, device }
            }

            #[inline]
            pub fn handle(&self) -> $handle {
                self.handle
            }

            #[inline]
            pub fn device(&self) -> &$crate::raii::Device {
                &self.device
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.handle)
                    .finish()
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { ::ash::Device::$destroy(self.device.handle(), self.handle, None) };
            }
        }
    };

    ($name:ident, $handle:ty, $destroy:ident, $create:ident($info:ty)) => {
        device_object!($name, $handle, $destroy);

        impl $crate::raii::Device {
            /// # Safety
            /// Same contract as the `ash` call it forwards to.
            #[inline]
            pub unsafe fn $create(&self, create_info: &$info) -> ::ash::prelude::VkResult<$name> {
                let handle = unsafe { ::ash::Device::$create(self.handle(), create_info, None) }?;
                Ok($name {
                    handle,
                    device: self.clone(),
                })
            }
        }
    };
}
