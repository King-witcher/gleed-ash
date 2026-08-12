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
        // Both bodies sit ON the rotation axis, so they spin in place instead of
        // orbiting each other: the ball stays nearer the camera than the cube
        // forever. That is what keeps the scene correct while there is still no
        // depth buffer — each body is convex, so back-face culling resolves it
        // internally, and the farther one is simply drawn first.
        let axis = Vec3::new(0.0, 1.0, 1.0).normalize();

        let meshes = vec![
            make_cube(&mut self.transfer, -0.45 * axis, 0.24)?,
            make_truncated_icosahedron(&mut self.transfer, 0.45 * axis, 0.30)?,
        ];

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

/// Axis-aligned cube, one flat color per face.
///
/// Each face gets its own four vertices because the color is per-vertex and the
/// six faces meeting at a corner disagree on it. Winding is CCW seen from
/// outside, matching the pipeline's front face.
fn make_cube(transfer: &mut TransferContext, center: Vec3, half_extent: f32) -> Result<Mesh> {
    let h = half_extent;

    // (four corners in CCW order seen from outside, face color)
    let faces = [
        // +X
        (
            [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]],
            [0.8, 0.2, 0.2],
        ),
        // -X
        (
            [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]],
            [0.4, 0.1, 0.1],
        ),
        // +Y
        (
            [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]],
            [0.2, 0.8, 0.2],
        ),
        // -Y
        (
            [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]],
            [0.1, 0.4, 0.1],
        ),
        // +Z
        (
            [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]],
            [0.2, 0.2, 0.8],
        ),
        // -Z
        (
            [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]],
            [0.1, 0.1, 0.4],
        ),
    ];

    let mut vertices = Vec::with_capacity(faces.len() * 4);
    let mut indices = Vec::with_capacity(faces.len() * 6);

    for (corners, color) in faces {
        let base = vertices.len() as u32;

        for corner in corners {
            vertices.push(Vertex {
                position: center + Vec3::from_array(corner),
                color: Vec3::from_array(color),
            });
        }

        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    Mesh::new(transfer, &vertices, &indices)
}

/// Truncated icosahedron — the football: 12 pentagons and 20 hexagons, every
/// vertex on the sphere of `radius`.
///
/// Built by cutting each icosahedron corner off at one third of every edge, so
/// each of the 20 triangles becomes a hexagon and each of the 12 corners becomes
/// a pentagon. The icosahedron's topology is derived from its corner positions
/// rather than a hand-written index table, which is also what gets the winding
/// right: every face comes out CCW seen from outside.
fn make_truncated_icosahedron(
    transfer: &mut TransferContext,
    center: Vec3,
    radius: f32,
) -> Result<Mesh> {
    const PENTAGON_COLOR: [f32; 3] = [0.04, 0.04, 0.05];
    const HEXAGON_COLOR: [f32; 3] = [0.90, 0.90, 0.87];

    // Where the corner at `from` gets cut off the edge `from`-`to`.
    let cut = |from: Vec3, to: Vec3| from + (to - from) / 3.0;

    let corners = icosahedron_corners();
    let rings: Vec<[usize; 5]> = (0..corners.len())
        .map(|index| neighbour_ring(&corners, index))
        .collect();

    let mut faces: Vec<(Vec<Vec3>, [f32; 3])> = Vec::with_capacity(32);

    for (index, ring) in rings.iter().enumerate() {
        let a = corners[index];

        // The corner itself becomes a pentagon: cut every edge leaving it.
        faces.push((
            ring.iter().map(|&n| cut(a, corners[n])).collect(),
            PENTAGON_COLOR,
        ));

        // Two consecutive ring entries close an icosahedron triangle, which
        // becomes a hexagon. Each triangle turns up once per corner, so only its
        // lowest-numbered corner emits it.
        for i in 0..ring.len() {
            let (j, k) = (ring[i], ring[(i + 1) % ring.len()]);
            if index > j || index > k {
                continue;
            }

            let (b, c) = (corners[j], corners[k]);
            faces.push((
                vec![
                    cut(a, b),
                    cut(b, a),
                    cut(b, c),
                    cut(c, b),
                    cut(c, a),
                    cut(a, c),
                ],
                HEXAGON_COLOR,
            ));
        }
    }

    let mut vertices = Vec::with_capacity(180);
    let mut indices = Vec::with_capacity(348);

    for (polygon, color) in faces {
        let base = vertices.len() as u32;

        // Every vertex of a truncated icosahedron is the same distance from the
        // center, so normalizing here is a uniform scale onto the sphere.
        for point in &polygon {
            vertices.push(Vertex {
                position: center + point.normalize() * radius,
                color: Vec3::from_array(color),
            });
        }

        // The faces are convex, so a fan triangulates them and keeps the winding.
        for i in 1..polygon.len() as u32 - 1 {
            indices.extend_from_slice(&[base, base + i, base + i + 1]);
        }
    }

    Mesh::new(transfer, &vertices, &indices)
}

/// The 12 icosahedron corners: the cyclic permutations of (0, ±1, ±phi), which
/// put every edge at length 2.
fn icosahedron_corners() -> [Vec3; 12] {
    const PHI: f32 = 1.618_034;

    let mut corners = [Vec3::ZERO; 12];

    for (i, chunk) in corners.chunks_exact_mut(3).enumerate() {
        let s = if i & 1 == 0 { 1.0 } else { -1.0 };
        let t = if i & 2 == 0 { 1.0 } else { -1.0 };

        chunk[0] = Vec3::new(0.0, s, t * PHI);
        chunk[1] = Vec3::new(s, t * PHI, 0.0);
        chunk[2] = Vec3::new(s * PHI, 0.0, t);
    }

    corners
}

/// The five corners adjacent to `corners[index]` — an icosahedron corner has
/// exactly five, and they are its five nearest — ordered counter-clockwise as
/// seen from outside.
fn neighbour_ring(corners: &[Vec3; 12], index: usize) -> [usize; 5] {
    let corner = corners[index];

    // `u` and `v` span the plane facing outwards, with u x v = normal, so a
    // rising atan2 in that basis walks the ring counter-clockwise from outside.
    let normal = corner.normalize();
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    let angle = |i: usize| {
        let offset = corners[i] - corner;
        offset.dot(v).atan2(offset.dot(u))
    };

    let mut ring: Vec<usize> = (0..corners.len()).filter(|&i| i != index).collect();
    ring.sort_by(|&a, &b| {
        corner
            .distance_squared(corners[a])
            .total_cmp(&corner.distance_squared(corners[b]))
    });
    ring.truncate(5);
    ring.sort_by(|&a, &b| angle(a).total_cmp(&angle(b)));

    ring.try_into().expect("truncated to five just above")
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
        .map_err(|error| Error::unsupported(format!("load the Vulkan loader: {error}")))?;

    let instance = unsafe { vk::raii::Instance::new(entry, &create_info) }
        .context("create the Vulkan instance")?;
    println!("Vulkan instance created.");

    Ok(instance)
}
