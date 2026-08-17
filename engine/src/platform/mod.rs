//! Everything that is not Vulkan: the window and the input it produces.
//! Equivalent to modules/engine/src/window.{h,cc} + input.{h,cc}.
//!
//! Same role the `vk` crate plays for Vulkan, on the other side: this is the
//! workspace's single door to SDL3, so no other module imports `sdl3`.

pub mod input;
mod window;

// `MouseVector` and `Size` are returned by methods that mirror the C++ API and
// have no caller in this demo yet — the same reason as the crate-level
// `allow(dead_code)`.
#[allow(unused_imports)]
pub use input::{Input, MouseButton, MouseVector, Scancode};
#[allow(unused_imports)]
pub use window::{Size, Window};
