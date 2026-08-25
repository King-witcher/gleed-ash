use std::marker::PhantomData;

use bytemuck::Pod;

use crate::memory::{allocator::RaiiAllocation, AllocMode, HostVisible};

/// Represents a buffer + it's bound memory allocation
#[derive(Debug)]
pub struct AllocatedBuffer<Mode: AllocMode> {
    vk_buffer: vk::raii::Buffer,
    allocation: RaiiAllocation,

    mode: PhantomData<Mode>,
}

impl<Mode: AllocMode> AllocatedBuffer<Mode> {
    pub(super) fn from_parts(allocation: RaiiAllocation, vk_buffer: vk::raii::Buffer) -> Self {
        Self {
            allocation,
            vk_buffer,
            mode: PhantomData,
        }
    }

    pub fn vk_buffer(&self) -> &vk::raii::Buffer {
        &self.vk_buffer
    }
}

impl AllocatedBuffer<HostVisible> {
    pub fn map_copy(&mut self, data: &[u8]) {
        let mapped = self
            .allocation
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
