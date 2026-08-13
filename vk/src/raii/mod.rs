//! Thin RAII wrappers over `ash`, in the spirit of `vulkan_raii.hpp`.
//!
//! Each wrapper owns its Vulkan handle, destroys it on `Drop`, and `Deref`s to
//! the underlying `ash` object — so the whole `ash` API stays reachable and
//! nothing needs re-wrapping method by method. What this module adds are the
//! constructors that return a wrapper instead of a bare handle, and the
//! methods that need a parent the `ash` type does not carry
//! ([`PhysicalDevice`], [`CommandBuffer`]).
//!
//! Nothing here validates Vulkan usage. Methods are `unsafe` whenever the call
//! they forward is, and every obligation stays with the caller: external
//! synchronization, object state, matching parents, valid parameters.
//!
//! The only thing this module guarantees is **destruction order**. Each child
//! holds a refcounted handle to its parent, so a `vkDestroy*` never runs after
//! the parent is gone — and no lifetime leaks into the API, which is what lets
//! a struct own a device and the objects created from it.
//!
//! Every wrapper exposes `handle()`, returning the matching `ash` handle.

#[macro_use]
mod macros;

mod command_buffer;
mod command_pool;
mod device;
mod instance;
mod objects;
mod physical_device;
mod queue;
mod surface;
mod swapchain;

pub use command_buffer::CommandBuffer;
pub use command_pool::CommandPool;
pub use device::Device;
pub use instance::Instance;
pub use objects::{
    DescriptorPool, DescriptorSetLayout, Fence, ImageView, Pipeline, PipelineLayout, Semaphore,
    ShaderModule,
};
pub use physical_device::PhysicalDevice;
pub use queue::Queue;
pub use surface::Surface;
pub use swapchain::Swapchain;
