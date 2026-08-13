//! What every engine module imports with `use crate::prelude::*`.
//!
//! Deliberately does not reexport `vk::VkResult`: it is the **raw** return of
//! each Vulkan call and must not escape the call site. Its path to the
//! engine's `Result` is always [`Context`]'s `.context(..)`, and no function
//! in the crate returns `VkResult` — if one did again, it would be a sign an
//! error is being passed along without saying which call failed.

pub use crate::error::{Context, Error, Result};
