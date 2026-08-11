//! Equivalente a modules/engine/src/engine.cc + include/engine/engine.h.
//!
//! O C++ usava o idioma pImpl (`Engine::Impl`) para esconder Vulkan/SDL do
//! header público. Em Rust isso não é necessário: `pub` por item já controla o
//! que sai da crate, e não existe header para vazar includes.

use std::ffi::{c_char, CString};
use std::rc::Rc;

use glam::Vec3;

use crate::allocator::Allocator;
use crate::device::{Device, API_VERSION_1_4};
use crate::input::Input;
use crate::mesh::Mesh;
use crate::prelude::*;
use crate::renderer::Renderer;
use crate::swapchain::Swapchain;
use crate::transfer::TransferContext;
use crate::vertex::Vertex;
use crate::window::Window;

pub struct Engine {
    // A ORDEM DOS CAMPOS É A ORDEM DE DESTRUIÇÃO. Tudo que usa o device vem
    // antes dele. A instance não aparece aqui: `Device` e `Swapchain` (via
    // `Surface`) seguram um clone dela, então ela morre por último sozinha.
    renderer: Renderer,
    swapchain: Swapchain,
    transfer: TransferContext,
    allocator: Rc<Allocator>,
    device: Device,
    input: Input,
    window: Window,
}

impl Engine {
    pub fn new() -> Result<Self> {
        let mut window = Window::new("Giuseppe")?;
        window.set_position(-1400, 200);

        // `LoadingError` não é erro de Vulkan nem de SDL, então não tem `Context`.
        let vulkan = create_instance(&window)?;

        let surface = {
            let handle = unsafe { window.vulkan_surface(vulkan.handle()) }?;
            unsafe { vk::raii::Surface::from_raw(vulkan.clone(), handle) }
        };

        let device = Device::new(&vulkan, &surface)?;
        let allocator = Rc::new(Allocator::new(device.clone())?);
        let transfer = TransferContext::new(device.clone(), Rc::clone(&allocator))?;
        let swapchain = Swapchain::new(device.clone(), surface)?;
        let input = Input::new(window.sdl())?;
        let renderer = Renderer::new(device.clone(), Rc::clone(&allocator), &swapchain)?;

        Ok(Self {
            renderer,
            swapchain,
            transfer,
            allocator,
            device,
            input,
            window,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let meshes = vec![make_cube(&mut self.transfer)?];

        loop {
            self.draw(&meshes)?;
            self.input.update();
            if self.input.should_quit() {
                break;
            }
        }

        self.device.wait_idle()?;
        println!("Exiting engine loop.");

        Ok(())
    }

    fn draw(&mut self, meshes: &[Mesh]) -> Result<()> {
        // Resolve o TODO que ficou em Swapchain::Recreate no C++: minimizada, a
        // surface tem extent 0x0 e recriar a swapchain seria inválido.
        if self.input.minimized() {
            return Ok(());
        }

        // `frame` empresta o renderer; a swapchain é campo disjunto, então
        // continua acessível para ler a extent e para o present no submit.
        let mut frame = self.renderer.begin_frame(&mut self.swapchain)?;
        let extent = self.swapchain.extent();
        frame.draw_scene(meshes, extent);
        frame.submit(&mut self.swapchain)?;

        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Roda antes dos campos serem destruídos, então o device ainda é válido.
        // `.ok()` para não entrar em pânico dentro de um Drop.
        self.device.wait_idle().ok();
    }
}

/// Unit cube, one flat color per face.
///
/// Each face gets its own four vertices because the color is per-vertex and the
/// six faces meeting at a corner disagree on it. Winding is CCW seen from
/// outside, matching the pipeline's front face.
///
/// There is no depth buffer yet: this renders correctly only because the cube is
/// convex and back faces are culled, so no two visible faces ever overlap.
fn make_cube(transfer: &mut TransferContext) -> Result<Mesh> {
    const H: f32 = 0.5;

    // (four corners in CCW order seen from outside, face color)
    let faces = [
        // +X
        (
            [[H, -H, H], [H, -H, -H], [H, H, -H], [H, H, H]],
            [0.8, 0.2, 0.2],
        ),
        // -X
        (
            [[-H, -H, -H], [-H, -H, H], [-H, H, H], [-H, H, -H]],
            [0.4, 0.1, 0.1],
        ),
        // +Y
        (
            [[-H, H, H], [H, H, H], [H, H, -H], [-H, H, -H]],
            [0.2, 0.8, 0.2],
        ),
        // -Y
        (
            [[-H, -H, -H], [H, -H, -H], [H, -H, H], [-H, -H, H]],
            [0.1, 0.4, 0.1],
        ),
        // +Z
        (
            [[-H, -H, H], [H, -H, H], [H, H, H], [-H, H, H]],
            [0.2, 0.2, 0.8],
        ),
        // -Z
        (
            [[H, -H, -H], [-H, -H, -H], [-H, H, -H], [H, H, -H]],
            [0.1, 0.1, 0.4],
        ),
    ];

    let mut vertices = Vec::with_capacity(faces.len() * 4);
    let mut indices = Vec::with_capacity(faces.len() * 6);

    for (corners, color) in faces {
        let base = vertices.len() as u32;

        for corner in corners {
            vertices.push(Vertex {
                position: Vec3::from_array(corner),
                color: Vec3::from_array(color),
            });
        }

        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    Mesh::new(transfer, &vertices, &indices)
}

fn create_instance(window: &Window) -> Result<vk::raii::Instance> {
    let app_info = vk::ApplicationInfo::default()
        .application_name(c"GLEED Test")
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(c"GLEED 1")
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(API_VERSION_1_4);

    let layers: Vec<CString> = if cfg!(debug_assertions) {
        println!("Enabling validation layers...");
        // Literal nosso, sem byte nulo no meio: só falharia se alguém o editasse errado.
        vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
    } else {
        Vec::new()
    };
    let layer_ptrs: Vec<*const c_char> = layers.iter().map(|name| name.as_ptr()).collect();

    // TODO (herdado do C++): checar as extensões suportadas.
    let required_extensions = window.required_vulkan_extensions()?;
    let extension_ptrs: Vec<*const c_char> = required_extensions
        .iter()
        .map(|name| name.as_ptr())
        .collect();

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_ptrs)
        .enabled_extension_names(&extension_ptrs);

    // `LoadingError` não é erro de Vulkan nem de SDL, então não passa pelo
    // `Context`: não há VkResult nenhum, só a ausência do loader na máquina.
    let entry = unsafe { vk::Entry::load() }
        .map_err(|error| Error::unsupported(format!("carregar o loader do Vulkan: {error}")))?;

    let instance = unsafe { vk::raii::Instance::new(entry, &create_info) }
        .context("criar a instance do Vulkan")?;
    println!("Vulkan instance created.");

    Ok(instance)
}
