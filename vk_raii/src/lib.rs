//! Thin RAII wrappers over `ash`, in the spirit of `vulkan_raii.hpp`.
//!
//! Every wrapper owns its Vulkan handle, destroys it on drop, and derefs to the
//! underlying `ash` object — so the whole `ash` API stays reachable and nothing
//! has to be re-wrapped method by method. What the crate adds on top are the
//! constructors that hand back a wrapper instead of a bare handle, and the
//! methods that need a parent the `ash` type does not carry (`PhysicalDevice`,
//! `CommandBuffer`).
//!
//! Nothing here validates Vulkan usage. Methods are `unsafe` wherever the call
//! they forward to is, and every obligation stays with the caller: external
//! synchronization, object state, matching parents, valid parameters.
//!
//! The one thing the crate does guarantee is **destruction order**. Each child
//! keeps a refcounted handle to its parent, so a `vkDestroy*` can never run
//! after its parent is gone — and no lifetime parameter leaks into the API,
//! which is what lets a struct own a device and the objects made from it.

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

pub use ash;
pub use ash::prelude::VkResult;
pub use ash::vk;

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
