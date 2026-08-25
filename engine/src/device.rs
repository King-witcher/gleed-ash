//! Equivalent to modules/engine/src/device.{h,cc}.
//!
//! Core difference from C++: `vulkan_raii.hpp` destroyed everything on its
//! own. The raw bindings are 1:1 with the C API — there is no RAII. That role
//! falls to the `vk::raii` module, a thin wrapper: it guarantees destruction
//! order and nothing else, so the calls stay `unsafe` and this is where the
//! engine takes on their contracts.
//!
//! `vk::raii::Device` is a cloneable handle with the refcount inside, and
//! every object created from it (semaphore, fence, shader module, queue)
//! carries a clone. `vkDestroyDevice` therefore only happens once the last of
//! those objects dies, with no manual destruction order as in C++.

use std::ffi::CStr;

use crate::internal_prelude::*;

/// The engine requires Vulkan 1.4. The bindings are generated from the 1.3
/// headers, so the 1.4 constant does not exist and the version is built by
/// hand.
pub const API_VERSION_1_4: u32 = vk::make_api_version(0, 1, 4, 0);

/// Same REQUIRED_EXTENSIONS list as device.cc.
pub const REQUIRED_EXTENSIONS: [&CStr; 5] = [
    vk::KHR_SHADER_DRAW_PARAMETERS_NAME,
    vk::KHR_CREATE_RENDERPASS2_NAME,
    vk::KHR_SYNCHRONIZATION2_NAME,
    vk::KHR_SWAPCHAIN_NAME,
    vk::KHR_SPIRV_1_4_NAME,
];

pub struct SurfaceSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

#[derive(Clone, Debug)]
pub struct Device {
    device: vk::raii::Device,
    graphics_index: u32,
    present_index: u32,
}

impl Device {
    pub fn new(vulkan: &vk::raii::Instance, surface: &vk::raii::Surface) -> Result<Self> {
        // The `PhysicalDevice` already carries the instance that enumerated
        // it, so the helpers below do not need to receive it again.
        let physical_device = pick_physical_device(vulkan)?;

        if cfg!(debug_assertions) {
            inspect_device(&physical_device);
        }

        let graphics_index = find_graphics_queue_family(surface, &physical_device)?;
        // Same as C++: one family serves both graphics and presentation.
        let present_index = graphics_index;

        let device = create_logical_device(&physical_device, graphics_index)?;

        Ok(Self {
            device,
            graphics_index,
            present_index,
        })
    }

    /// Surface support information for the physical device.
    ///
    /// The three queries go through the `PhysicalDevice`, which uses the
    /// `Surface`'s own loader — the `Device` no longer keeps a loader just for
    /// this.
    pub fn query_surface_support(&self, surface: &vk::raii::Surface) -> Result<SurfaceSupport> {
        let physical_device = self.physical_device();

        // The surface came from the same instance that enumerated this
        // physical device: both come out of `Engine::new`.
        unsafe {
            let capabilities = physical_device
                .surface_capabilities(surface)
                .context("get the physical device surface capabilities")?;
            let formats = physical_device
                .surface_formats(surface)
                .context("get the physical device surface formats")?;
            let present_modes = physical_device
                .surface_present_modes(surface)
                .context("get the physical device surface present modes")?;

            Ok(SurfaceSupport {
                capabilities,
                formats,
                present_modes,
            })
        }
    }

    pub fn create_fence(&self, signaled: bool) -> Result<vk::raii::Fence> {
        let flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };

        unsafe {
            self.device
                .create_fence(&vk::FenceCreateInfo::default().flags(flags))
        }
        .context("create the fence")
    }

    pub fn create_semaphore(&self) -> Result<vk::raii::Semaphore> {
        unsafe {
            self.device
                .create_semaphore(&vk::SemaphoreCreateInfo::default())
        }
        .context("create the semaphore")
    }

    pub fn graphics_index(&self) -> u32 {
        self.graphics_index
    }

    pub fn present_index(&self) -> u32 {
        self.present_index
    }

    pub fn vk_device(&self) -> &vk::raii::Device {
        &self.device
    }

    pub fn physical_device(&self) -> &vk::raii::PhysicalDevice {
        self.device.physical_device()
    }

    /// The queue owns the device by refcount: the `vkQueue*` calls come from
    /// its dispatch table.
    pub fn queue(&self, family_index: u32) -> vk::raii::Queue {
        // The family came out of this device's `VkDeviceCreateInfo`, and only
        // one queue was requested for it.
        unsafe { self.device.queue(family_index, 0) }
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.device.handle().device_wait_idle() }
            .context("wait for the device to become idle")
    }
}

fn inspect_device(physical_device: &vk::raii::PhysicalDevice) {
    let (properties, mem_properties) = unsafe {
        (
            physical_device.properties(),
            physical_device.memory_properties(),
        )
    };

    // `device_name_as_c_str` does what the old `CStr::from_ptr` did, but
    // looking for the terminator inside the array — no `unsafe`, and no
    // out-of-bounds read if the driver returns a name without a NUL byte.
    let name = properties
        .device_name_as_c_str()
        .unwrap_or(c"<nome inválido>")
        .to_string_lossy();
    println!("Found device: {} - {}", name, properties.device_id);
    println!(
        "Max memory allocation count: {}",
        properties.limits.max_memory_allocation_count
    );
    println!("Memory heaps: {}", mem_properties.memory_heap_count);
    println!("Memory types: {}", mem_properties.memory_type_count);

    for heap_index in 0..mem_properties.memory_heap_count {
        let heap = mem_properties.memory_heaps[heap_index as usize];
        println!(
            "  Heap {}: {} GB, flags: {:?}",
            heap_index,
            heap.size as f64 / (1024.0 * 1024.0 * 1024.0),
            heap.flags
        );

        for type_index in 0..mem_properties.memory_type_count {
            let ty = mem_properties.memory_types[type_index as usize];
            if ty.heap_index != heap_index {
                continue;
            }
            println!("    Memory type {}: {:?}", type_index, ty.property_flags);
        }
    }
}

fn is_device_suitable(device: &vk::raii::PhysicalDevice) -> bool {
    let (properties, features) = unsafe { (device.properties(), device.features()) };

    if properties.api_version < API_VERSION_1_4 {
        return false;
    }
    if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
        return false;
    }
    if features.geometry_shader == vk::FALSE {
        return false;
    }
    true
}

fn pick_physical_device(vulkan: &vk::raii::Instance) -> Result<vk::raii::PhysicalDevice> {
    let devices =
        unsafe { vulkan.enumerate_physical_devices() }.context("enumerate the physical devices")?;
    if devices.is_empty() {
        return Err(Error::unsupported("no GPU with Vulkan support"));
    }

    let found = devices.len();

    // TODO: Pick the most suitable device
    devices.into_iter().find(is_device_suitable).ok_or_else(|| {
        Error::unsupported(format!(
            "none of the {found} GPUs is discrete with Vulkan 1.4 and a geometry shader"
        ))
    })
}

fn find_graphics_queue_family(
    surface: &vk::raii::Surface,
    device: &vk::raii::PhysicalDevice,
) -> Result<u32> {
    let family_properties = unsafe { device.queue_family_properties() };

    for (i, properties) in family_properties.iter().enumerate() {
        let i = i as u32;
        if !properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            continue;
        }

        // The surface and the physical device come from the same instance,
        // created in `Engine::new`.
        let present_supported = unsafe { device.surface_support(surface, i) }
            .context("query surface support for the queue family")?;

        if present_supported {
            return Ok(i);
        }
    }

    Err(Error::unsupported(
        "no queue family supports both graphics and presentation for this surface",
    ))
}

fn create_logical_device(
    physical_device: &vk::raii::PhysicalDevice,
    queue_family_index: u32,
) -> Result<vk::raii::Device> {
    let queue_priorities = [0.5f32];
    let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&queue_priorities)];

    // Equivalent to C++'s vk::StructureChain: the pNext chain is assembled
    // with push_next(), and the borrow checker guarantees each chained struct
    // stays alive until the call.
    let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default()
        .synchronization2(true)
        .dynamic_rendering(true);
    let mut extended_dynamic_state =
        vk::PhysicalDeviceExtendedDynamicStateFeaturesEXT::default().extended_dynamic_state(true);
    let mut features = vk::PhysicalDeviceFeatures2::default()
        .push_next(&mut vulkan13_features)
        .push_next(&mut extended_dynamic_state);

    let extension_names: Vec<*const i8> = REQUIRED_EXTENSIONS
        .iter()
        .map(|name| name.as_ptr())
        .collect();

    let device_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&extension_names)
        .push_next(&mut features);

    unsafe { physical_device.create_device(&device_info) }.context("create the device")
}
