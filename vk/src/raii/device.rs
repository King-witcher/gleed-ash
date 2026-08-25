use std::fmt;
use std::rc::Rc;

use ash::khr;
use ash::prelude::VkResult;
use ash::vk;

use super::{CommandPool, PhysicalDevice, Pipeline, Queue, Swapchain};

#[derive(Clone)]
pub struct Device(Rc<DeviceInner>);

struct DeviceInner {
    handle: ash::Device,
    physical_device: PhysicalDevice,
}

impl Device {
    #[inline]
    pub unsafe fn from_handle(physical_device: PhysicalDevice, handle: ash::Device) -> Self {
        Self(Rc::new(DeviceInner {
            handle,
            physical_device,
        }))
    }

    #[inline]
    pub fn handle(&self) -> &ash::Device {
        &self.0.handle
    }

    #[inline]
    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.0.physical_device
    }

    #[inline]
    pub unsafe fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.0.handle.device_wait_idle() }
    }

    #[inline]
    pub unsafe fn queue(&self, queue_family_index: u32, queue_index: u32) -> Queue {
        let handle = unsafe {
            self.0
                .handle
                .get_device_queue(queue_family_index, queue_index)
        };
        Queue::from_handle(self.clone(), handle, queue_family_index)
    }

    pub unsafe fn create_command_pool(
        &self,
        create_info: &vk::CommandPoolCreateInfo,
    ) -> VkResult<CommandPool> {
        let handle = unsafe { self.0.handle.create_command_pool(create_info, None) }?;
        Ok(unsafe { CommandPool::from_handle(self.clone(), handle) })
    }

    pub unsafe fn create_swapchain(
        &self,
        create_info: &vk::SwapchainCreateInfoKHR,
    ) -> VkResult<Swapchain> {
        let loader =
            khr::swapchain::Device::new(self.physical_device().instance().handle(), self.handle());
        let handle = unsafe { loader.create_swapchain(create_info, None) }?;
        Ok(unsafe { Swapchain::from_handle(self.clone(), loader, handle) })
    }

    pub unsafe fn create_graphics_pipeline(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_info: vk::GraphicsPipelineCreateInfo,
    ) -> VkResult<Pipeline> {
        unsafe {
            let mut created = self
                .handle()
                .create_graphics_pipelines(pipeline_cache, &[create_info], None)
                .map_err(|(_, e)| e)?;
            let handle = created.pop().unwrap();
            Ok(Pipeline::from_handle(self.clone(), handle))
        }
    }

    pub unsafe fn update_descriptor_sets(
        &self,
        descriptor_writes: &[vk::WriteDescriptorSet],
        descriptor_copies: &[vk::CopyDescriptorSet],
    ) {
        unsafe {
            self.0
                .handle
                .update_descriptor_sets(descriptor_writes, descriptor_copies)
        }
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Device")
            .field(&self.0.handle.handle())
            .finish()
    }
}

impl PartialEq for Device {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Device {}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_device(None) };
    }
}
