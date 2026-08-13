//! A buffer and the memory backing it, as one RAII object.
//!
//! `Buffer` is still RAII as in C++: it frees itself on `Drop`, holding a
//! clone of the allocator — the analogue of the `VmaAllocator` that
//! `gd::Buffer` kept. What C++ could not express is the typestate: the memory
//! mode is a type parameter, and only [`Buffer<HostVisible>`] has the mapping
//! methods, so mapping VRAM is a compile error instead of a runtime panic.

use std::marker::PhantomData;

use bytemuck::Pod;
use gpu_allocator::vulkan::Allocation;
use gpu_allocator::MemoryLocation;

use super::allocator::Allocator;
use crate::device::Device;

/// CPU-mappable memory (staging, uniforms).
pub struct HostVisible;

/// Pure VRAM, no CPU access.
pub struct DeviceLocal;

/// Where a buffer's memory lives, lifted to the type level. Implemented only
/// by [`HostVisible`] and [`DeviceLocal`].
pub trait AllocMode {
    const LOCATION: MemoryLocation;
}

impl AllocMode for HostVisible {
    const LOCATION: MemoryLocation = MemoryLocation::CpuToGpu;
}

impl AllocMode for DeviceLocal {
    const LOCATION: MemoryLocation = MemoryLocation::GpuOnly;
}

pub struct Buffer<Mode: AllocMode> {
    /// Refcounted handle: the `vkDestroyBuffer` in the `Drop` below never runs
    /// after `vkDestroyDevice`.
    device: Device,
    allocator: Allocator,
    // Option only so the Allocation can be given back to the allocator in Drop.
    allocation: Option<Allocation>,
    buffer: vk::Buffer,
    mode: PhantomData<Mode>,
}

impl<Mode: AllocMode> Buffer<Mode> {
    /// Takes ownership of a buffer and the allocation backing it. Only
    /// [`Allocator::allocate`] builds one — it is the call that knows
    /// `allocation` came from `allocator`, matches `buffer`, and sits in the
    /// memory `Mode` describes.
    pub(super) fn from_parts(
        device: Device,
        allocator: Allocator,
        allocation: Allocation,
        buffer: vk::Buffer,
    ) -> Self {
        Self {
            device,
            allocator,
            allocation: Some(allocation),
            buffer,
            mode: PhantomData,
        }
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer
    }
}

impl Buffer<HostVisible> {
    pub fn map_copy(&mut self, data: &[u8]) {
        let allocation = self
            .allocation
            .as_mut()
            .expect("buffer has already been freed");
        let mapped = allocation
            .mapped_slice_mut()
            .expect("host-visible memory is always mapped");
        mapped[..data.len()].copy_from_slice(data);
    }

    pub fn map_copy_slice<T: Pod>(&mut self, values: &[T]) {
        self.map_copy(bytemuck::cast_slice(values));
    }

    pub fn map_copy_value<T: Pod>(&mut self, value: &T) {
        self.map_copy(bytemuck::bytes_of(value));
    }
}

impl<Mode: AllocMode> Drop for Buffer<Mode> {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            self.allocator.free(allocation);
        }
        unsafe { self.device.raw().destroy_buffer(self.buffer, None) };
    }
}
