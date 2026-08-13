//! Everything that owns GPU memory: the sub-allocator, the buffers it hands
//! out, and the staging path that fills them. Equivalent to
//! modules/engine/src/allocator.{h,cc} + transfer.{h,cc}.
//!
//! VMA is C++, so its role here falls to `gpu-allocator` (pure Rust): it
//! sub-allocates large `VkDeviceMemory` blocks and picks the memory type from
//! a declared usage, exactly like `VMA_MEMORY_USAGE_AUTO` + flags in C++.

mod allocator;
mod buffer;
mod transfer;

pub use allocator::Allocator;
// `AllocMode` has no user outside this module yet — callers name the concrete
// markers — but it is the bound on every `Buffer<Mode>`, so it belongs to the
// module's surface.
#[allow(unused_imports)]
pub use buffer::{AllocMode, Buffer, DeviceLocal, HostVisible};
pub use transfer::TransferContext;
