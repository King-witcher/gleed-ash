//! Vulkan for this workspace: the slice of `ash` we actually use, plus the
//! RAII layer on top of it.
//!
//! Three different things share each name, and keeping them apart matters:
//!
//! - [`Device`] — the raw `VkDevice`, an opaque integer. Can do nothing by
//!   itself.
//! - [`raw::Device`] — the loaded device from `ash`: that handle plus the
//!   dispatch table serving it. It is what knows how to call `vkCmdDraw`.
//! - [`raii::Device`] — what this crate adds: a `raw::Device` that owns
//!   itself, keeps its parent alive and destroys itself.
//!
//! Nothing outside this crate should depend on `ash` directly — whatever is
//! needed gets reexported here.

pub mod raii;

/// The *loaded* `ash` objects: a handle together with the dispatch table that
/// serves it. They live in a separate module because [`Device`] and
/// [`Instance`], at the root, are the bare handles.
pub mod raw {
    pub use ash::{Device, Instance};
}

pub use ash::prelude::VkResult;
pub use ash::vk::*;
pub use ash::{Entry, khr, util};
