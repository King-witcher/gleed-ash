//! Equivalente a modules/engine/src/allocator.{h,cc}.
//!
//! A VMA é C++, então aqui o papel dela é da `gpu-allocator` (Rust puro):
//! sub-aloca blocos grandes de `VkDeviceMemory` e escolhe o memory type a
//! partir de um "uso" declarado (`MemoryLocation`), exatamente como o
//! `VMA_MEMORY_USAGE_AUTO` + flags do C++.
//!
//! O `Buffer` continua sendo RAII como no C++: ele se auto-libera no `Drop`.
//! Para isso guarda um `Rc` do allocator — o análogo do `VmaAllocator` que o
//! `gd::Buffer` guardava.

use std::cell::RefCell;
use std::rc::Rc;

use ash::vk;
use gpu_allocator::vulkan::{
    Allocation, AllocationCreateDesc, AllocationScheme, Allocator as GpuAllocator,
    AllocatorCreateDesc,
};
use gpu_allocator::MemoryLocation;

use crate::device::Device;
use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AllocMode {
    /// mapeável pela CPU (staging, uniforms)
    HostVisible,
    /// VRAM pura, sem acesso da CPU
    DeviceLocal,
}

/// Reinterpreta qualquer fatia `Copy` como bytes — o que no C++ era feito com
/// `reinterpret_cast<const u8*>` nos templates de `MapCopy`/`UploadBuffer`.
pub fn as_bytes<T: Copy>(values: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr() as *const u8, std::mem::size_of_val(values))
    }
}

pub fn value_as_bytes<T: Copy>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub struct Buffer {
    /// Handle refcontado: o `vkDestroyBuffer` do `Drop` abaixo nunca roda
    /// depois do `vkDestroyDevice`.
    device: Device,
    allocator: Rc<RefCell<GpuAllocator>>,
    // Option só para poder devolver a Allocation ao allocator no Drop.
    allocation: Option<Allocation>,
    buffer: vk::Buffer,
}

impl Buffer {
    pub fn map_copy(&mut self, data: &[u8]) {
        let allocation = self
            .allocation
            .as_mut()
            .expect("buffer has already been freed");
        let mapped = allocation
            .mapped_slice_mut()
            .expect("buffer is not host visible");
        mapped[..data.len()].copy_from_slice(data);
    }

    pub fn map_copy_slice<T: Copy>(&mut self, values: &[T]) {
        self.map_copy(as_bytes(values));
    }

    pub fn map_copy_value<T: Copy>(&mut self, value: &T) {
        self.map_copy(value_as_bytes(value));
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            let _ = self.allocator.borrow_mut().free(allocation);
        }
        unsafe { self.device.raw().destroy_buffer(self.buffer, None) };
    }
}

pub struct Allocator {
    device: Device,
    inner: Rc<RefCell<GpuAllocator>>,
}

impl Allocator {
    pub fn new(device: Device) -> Result<Self> {
        let inner = GpuAllocator::new(&AllocatorCreateDesc {
            instance: device.vulkan_raw().clone(),
            device: device.raw().clone(),
            physical_device: device.physical_device_handle(),
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .context("criar o allocator de GPU")?;

        Ok(Self {
            device,
            inner: Rc::new(RefCell::new(inner)),
        })
    }

    pub fn allocate(&self, buffer_info: &vk::BufferCreateInfo, mode: AllocMode) -> Result<Buffer> {
        let raw = self.device.raw();

        let buffer = unsafe { raw.create_buffer(buffer_info, None) }.context("criar buffer")?;
        let requirements = unsafe { raw.get_buffer_memory_requirements(buffer) };

        // HostVisible: memória mapeável para a CPU escrever (staging/uniforms).
        // DeviceLocal: sem acesso do host, então sobra VRAM pura.
        let location = match mode {
            AllocMode::HostVisible => MemoryLocation::CpuToGpu,
            AllocMode::DeviceLocal => MemoryLocation::GpuOnly,
        };

        let allocation = self
            .inner
            .borrow_mut()
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            // O `vk::Buffer` acima já existe: sem desfazê-lo aqui, ele vazaria.
            .inspect_err(|_| unsafe { raw.destroy_buffer(buffer, None) })
            .context("alocar memória do buffer")?;

        // Montar o `Buffer` antes do bind é de propósito: se o bind falhar, é o
        // `Drop` dele que devolve a alocação e destrói o `vk::Buffer`.
        let (memory, offset) = (unsafe { allocation.memory() }, allocation.offset());
        let allocated = Buffer {
            device: self.device.clone(),
            allocator: Rc::clone(&self.inner),
            allocation: Some(allocation),
            buffer,
        };

        unsafe { raw.bind_buffer_memory(buffer, memory, offset) }
            .context("fazer bind da memória do buffer")?;

        Ok(allocated)
    }
}
