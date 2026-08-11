//! Equivalente a modules/engine/src/device.{h,cc}.
//!
//! Diferença central em relação ao C++: o `vulkan_raii.hpp` destruía tudo
//! sozinho. As bindings cruas são 1:1 com a API C — não há RAII. Quem cobre
//! esse papel é o módulo `vk::raii`, um wrapper fino: ele garante a ordem de
//! destruição e nada além disso, então as chamadas continuam `unsafe` e é aqui
//! que o engine assume o contrato delas.
//!
//! O `vk::raii::Device` é um handle clonável com o refcount por dentro, e cada
//! objeto criado a partir dele (semáforo, fence, shader module, queue) carrega
//! um clone. Assim o `vkDestroyDevice` só acontece quando o último desses
//! objetos morre, sem depender de ordem manual de destruição como no C++.

use std::ffi::CStr;

use crate::prelude::*;

/// O engine exige Vulkan 1.4. As bindings são geradas em cima dos headers 1.3,
/// então a constante 1.4 não existe e montamos a versão na mão.
pub const API_VERSION_1_4: u32 = vk::make_api_version(0, 1, 4, 0);

/// Mesma lista de REQUIRED_EXTENSIONS do device.cc.
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

/// Handle clonável para o device lógico: clonar custa um refcount do
/// `vk::raii::Device` mais os dois índices de queue family.
#[derive(Clone)]
pub struct Device {
    device: vk::raii::Device,
    graphics_index: u32,
    present_index: u32,
}

impl Device {
    pub fn new(vulkan: &vk::raii::Instance, surface: &vk::raii::Surface) -> Result<Self> {
        // O `PhysicalDevice` já carrega a instance que o enumerou, então os
        // helpers abaixo não precisam recebê-la de novo.
        let physical_device = pick_physical_device(vulkan)?;

        if cfg!(debug_assertions) {
            inspect_device(&physical_device);
        }

        let graphics_index = find_graphics_queue_family(surface, &physical_device)?;
        // Igual ao C++: a mesma família serve para gráficos e apresentação.
        let present_index = graphics_index;

        let device = create_logical_device(&physical_device, graphics_index)?;

        Ok(Self {
            device,
            graphics_index,
            present_index,
        })
    }

    /// Informações de suporte da surface para o physical device.
    ///
    /// As três consultas saem do `PhysicalDevice`, que usa o loader da própria
    /// `Surface` — o `Device` não guarda mais um loader só para isso.
    pub fn query_surface_support(&self, surface: &vk::raii::Surface) -> Result<SurfaceSupport> {
        let physical_device = self.physical_device();

        // A surface veio da mesma instance que enumerou este physical device:
        // as duas saem do `Engine::new`.
        unsafe {
            let capabilities = physical_device
                .surface_capabilities(surface)
                .context("obter as surface capabilities do physical device")?;
            let formats = physical_device
                .surface_formats(surface)
                .context("obter os surface formats do physical device")?;
            let present_modes = physical_device
                .surface_present_modes(surface)
                .context("obter os surface present modes do physical device")?;

            Ok(SurfaceSupport {
                capabilities,
                formats,
                present_modes,
            })
        }
    }

    pub fn create_command_pool(
        &self,
        create_info: &vk::CommandPoolCreateInfo,
    ) -> Result<vk::raii::CommandPool> {
        unsafe { self.device.create_command_pool(create_info) }.context("criar command pool")
    }

    pub fn create_semaphore(&self) -> Result<vk::raii::Semaphore> {
        unsafe {
            self.device
                .create_semaphore(&vk::SemaphoreCreateInfo::default())
        }
        .context("criar semáforo")
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
        .context("criar fence")
    }

    pub fn create_shader_module(&self, code: &[u32]) -> Result<vk::raii::ShaderModule> {
        unsafe {
            self.device
                .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(code))
        }
        .context("criar shader module")
    }

    pub fn graphics_index(&self) -> u32 {
        self.graphics_index
    }

    pub fn present_index(&self) -> u32 {
        self.present_index
    }

    /// O handle RAII. É por ele que os módulos criam objetos que se destroem
    /// sozinhos; as chamadas cruas chegam pelo `Deref` dele.
    pub fn vk(&self) -> &vk::raii::Device {
        &self.device
    }

    /// A tabela de dispatch, para as chamadas que não produzem um objeto RAII e
    /// para as bibliotecas que falam Vulkan por conta própria.
    pub fn raw(&self) -> &vk::raw::Device {
        &self.device
    }

    /// A instance que criou este device. Quem precisa dela pega daqui em vez de
    /// recebê-la de novo por fora.
    pub fn vulkan(&self) -> &vk::raii::Instance {
        self.device.instance()
    }

    /// A instance com a tabela de dispatch — hoje só a `gpu-allocator` precisa.
    pub fn vulkan_raw(&self) -> &vk::raw::Instance {
        self.device.instance()
    }

    pub fn physical_device(&self) -> &vk::raii::PhysicalDevice {
        self.device.physical_device()
    }

    /// O handle pelado, para as bibliotecas que falam Vulkan por conta própria
    /// — hoje a `gpu-allocator`, no `Allocator::new`.
    pub fn physical_device_handle(&self) -> vk::PhysicalDevice {
        self.physical_device().handle()
    }

    /// A queue é dona do device por refcount: os `vkQueue*` saem da tabela de
    /// dispatch dele.
    pub fn get_queue(&self, family_index: u32) -> vk::raii::Queue {
        // A família saiu do `VkDeviceCreateInfo` deste device e só pedimos uma
        // queue nela.
        unsafe { self.device.queue(family_index, 0) }
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.device.device_wait_idle() }.context("esperar o device ficar idle")
    }
}

fn inspect_device(physical_device: &vk::raii::PhysicalDevice) {
    let (properties, mem_properties) = unsafe {
        (
            physical_device.properties(),
            physical_device.memory_properties(),
        )
    };

    // `device_name_as_c_str` faz o mesmo que o `CStr::from_ptr` de antes, só que
    // procurando o terminador dentro do array — sem `unsafe`, e sem ler fora do
    // limite se o driver devolver o nome sem byte nulo.
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
        unsafe { vulkan.enumerate_physical_devices() }.context("enumerar os physical devices")?;
    if devices.is_empty() {
        return Err(Error::unsupported("nenhuma GPU com suporte a Vulkan"));
    }

    let found = devices.len();

    // TODO: Pick the most suitable device
    devices.into_iter().find(is_device_suitable).ok_or_else(|| {
        Error::unsupported(format!(
            "nenhuma das {found} GPUs é discreta com Vulkan 1.4 e geometry shader"
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

        // Surface e physical device saem da mesma instance, criada no
        // `Engine::new`.
        let present_supported = unsafe { device.surface_support(surface, i) }
            .context("consultar o suporte da surface para a queue family")?;

        if present_supported {
            return Ok(i);
        }
    }

    Err(Error::unsupported(
        "nenhuma queue family suporta gráficos e apresentação para esta surface",
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

    // Equivalente ao vk::StructureChain do C++: aqui a cadeia de pNext é
    // montada com push_next(), e o borrow checker garante que cada struct
    // encadeada continua viva até a chamada.
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

    unsafe { physical_device.create_device(&device_info) }.context("criar device")
}
