//! The sub-allocator every [`Buffer`] comes from.

use std::cell::RefCell;
use std::rc::Rc;

use gpu_allocator::vulkan::{
    Allocation, AllocationCreateDesc, AllocationScheme, Allocator as GpuAllocator,
    AllocatorCreateDesc,
};

use super::buffer::{AllocMode, Buffer};
use crate::device::Device;
use crate::internal_prelude::*;

struct AllocatorInner {
    device: Device,
    gpu_allocator: GpuAllocator,
}

/// Cheap cloneable handle, the same pattern as [`Device`]: the refcount lives
/// inside, shared with every [`Buffer`] allocated from here.
#[derive(Clone)]
pub struct Allocator(Rc<RefCell<AllocatorInner>>);

impl Allocator {
    pub fn new(device: Device) -> Result<Self> {
        let gpu_allocator = GpuAllocator::new(&AllocatorCreateDesc {
            instance: device.vulkan_raw().clone(),
            device: device.raw().clone(),
            physical_device: device.physical_device_handle(),
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .context("create the GPU allocator")?;

        Ok(Self(Rc::new(RefCell::new(AllocatorInner {
            device,
            gpu_allocator,
        }))))
    }

    pub fn allocate<Mode: AllocMode>(
        &self,
        buffer_info: &vk::BufferCreateInfo,
    ) -> Result<Buffer<Mode>> {
        // Cloned out of the cell so the borrow below is not held across the
        // `borrow_mut` the allocation needs.
        let device = self.0.borrow().device.clone();
        let raw = device.raw();

        let buffer =
            unsafe { raw.create_buffer(buffer_info, None) }.context("create the buffer")?;
        let requirements = unsafe { raw.get_buffer_memory_requirements(buffer) };

        let allocation = self
            .0
            .borrow_mut()
            .gpu_allocator
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location: Mode::LOCATION,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            // The `vk::Buffer` above already exists: without undoing it here,
            // it would leak.
            .inspect_err(|_| unsafe { raw.destroy_buffer(buffer, None) })
            .context("allocate the buffer memory")?;

        // Assembling the `Buffer` before the bind is deliberate: if the bind
        // fails, its `Drop` is what returns the allocation and destroys the
        // `vk::Buffer`.
        let (memory, offset) = (unsafe { allocation.memory() }, allocation.offset());
        let allocated = Buffer::from_parts(device.clone(), self.clone(), allocation, buffer);

        unsafe { raw.bind_buffer_memory(buffer, memory, offset) }
            .context("bind the buffer memory")?;

        Ok(allocated)
    }

    /// Gives an allocation back. Only [`Buffer`]'s `Drop` calls it, with the
    /// allocation this allocator handed out.
    pub(super) fn free(&self, allocation: Allocation) {
        self.0
            .borrow_mut()
            .gpu_allocator
            .free(allocation)
            .expect("failed to free memory")
    }
}
