use std::cell::RefCell;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use gpu_allocator::vulkan::{
    Allocation, AllocationCreateDesc, AllocationScheme, Allocator as GpuAllocator,
    AllocatorCreateDesc,
};
use gpu_allocator::MemoryLocation;

use super::buffer::AllocatedBuffer;
use crate::device::Device;
use crate::internal_prelude::*;
use crate::memory::image::AllocatedImage;

#[derive(Debug)]
pub struct HostVisible;

#[derive(Debug)]
pub struct DeviceLocal;

pub trait AllocMode {
    const LOCATION: MemoryLocation;
}

impl AllocMode for HostVisible {
    const LOCATION: MemoryLocation = MemoryLocation::CpuToGpu;
}

impl AllocMode for DeviceLocal {
    const LOCATION: MemoryLocation = MemoryLocation::GpuOnly;
}

#[derive(Debug)]
struct AllocatorInner {
    device: Device,
    gpu_allocator: GpuAllocator,
}

#[derive(Clone, Debug)]
pub struct Allocator(Rc<RefCell<AllocatorInner>>);

#[derive(Debug)]
pub struct RaiiAllocation {
    allocation: Allocation,
    allocator: Allocator,
}

impl Allocator {
    pub fn new(device: Device) -> Result<Self> {
        let gpu_allocator = GpuAllocator::new(&AllocatorCreateDesc {
            instance: device.physical_device().instance().handle().clone(),
            device: device.vk_device().handle().clone(),
            physical_device: device.physical_device().handle(),
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
        requirements: vk::MemoryRequirements,
        linear: bool,
        name: &str,
    ) -> Result<RaiiAllocation> {
        let raw = self
            .0
            .borrow_mut()
            .gpu_allocator
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location: Mode::LOCATION,
                linear,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .context("allocate buffer memory")?;

        Ok(RaiiAllocation {
            allocation: raw,
            allocator: self.clone(),
        })
    }

    pub fn create_buffer<Mode: AllocMode>(
        &self,
        buffer_info: &vk::BufferCreateInfo,
    ) -> Result<AllocatedBuffer<Mode>> {
        let device = self.0.borrow().device.clone();
        let vk_device = device.vk_device();

        let buffer =
            unsafe { vk_device.create_buffer(buffer_info) }.context("create the buffer")?;

        // Name the allocation
        let allocation =
            self.allocate::<Mode>(buffer.memory_requirements(), true, "unnamed buffer")?;

        unsafe { buffer.bind_memory(allocation.memory(), allocation.offset()) }
            .context("bind the buffer memory")?;

        let allocated = AllocatedBuffer::from_parts(allocation, buffer);

        Ok(allocated)
    }

    pub fn create_image(&self, create_info: &vk::ImageCreateInfo) -> Result<AllocatedImage> {
        let device = self.0.borrow().device.clone();
        let vk_device = device.vk_device();

        let mut image =
            unsafe { vk_device.create_image(&create_info) }.context("create vkImage")?;

        // Name the allocation
        let allocation = self.allocate::<DeviceLocal>(
            unsafe { image.memory_requirements() },
            create_info.tiling == vk::ImageTiling::LINEAR,
            "unnamed image",
        )?;

        unsafe { image.bind_memory(allocation.memory(), allocation.offset()) }
            .context("bind image to memory")?;

        let allocated =
            AllocatedImage::from_parts(image, allocation, create_info.format, create_info.extent);

        Ok(allocated)
    }

    fn free(&self, allocation: Allocation) {
        self.0
            .borrow_mut()
            .gpu_allocator
            .free(allocation)
            .expect("failed to free memory")
    }
}

impl Drop for RaiiAllocation {
    fn drop(&mut self) {
        let allocation = mem::take(&mut self.allocation);
        self.allocator.free(allocation);
    }
}

impl Deref for RaiiAllocation {
    type Target = Allocation;

    fn deref(&self) -> &Self::Target {
        &self.allocation
    }
}

impl DerefMut for RaiiAllocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.allocation
    }
}
