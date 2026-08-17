//! Equivalent to modules/engine/src/engine.cc + include/engine/engine.h.
//!
//! C++ used the pImpl idiom (`Engine::Impl`) to keep Vulkan/SDL out of the
//! public header. Rust needs none of that: per-item `pub` already controls
//! what leaves the crate, and there is no header to leak includes.
//!
//! The engine owns no scene: the caller builds [`MeshData`], turns it into GPU
//! meshes with [`Engine::upload_mesh`], and hands them to [`Engine::run`].

use std::ffi::{c_char, CString};

use glam::Vec3;

use crate::device::{Device, API_VERSION_1_4};
use crate::internal_prelude::*;
use crate::memory::{Allocator, TransferContext};
use crate::mesh::{Mesh, MeshData};
use crate::platform::{Input, Window};
use crate::renderer::{Camera, Renderer};
use crate::swapchain::Swapchain;
use crate::time::{Clock, Duration, TimeStep};

pub struct Engine {
    // FIELD ORDER IS DESTRUCTION ORDER. Everything that uses the device comes
    // before it. The instance is not a field: `Device` and `Swapchain` (via
    // `Surface`) hold a clone of it, so it dies last on its own. Neither is
    // the allocator: `TransferContext` and every live `Buffer` keep it alive
    // by refcount.
    renderer: Renderer,
    swapchain: Swapchain,
    transfer: TransferContext,
    device: Device,
    input: Input,
    window: Window,
}

pub struct FrameContext<'a> {
    pub camera_pos: Vec3,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub input: &'a Input,
    pub timestep: TimeStep,
}

pub struct RunInfo {
    pub update: Box<dyn FnMut(&mut FrameContext)>,
    pub scene: Vec<Mesh>,
}

impl Engine {
    pub fn new(window_title: &str) -> Result<Self> {
        let mut window = Window::new(window_title)?;
        window.set_relative_mouse_mode(true);

        let vulkan = create_instance(&window)?;

        let surface = {
            let handle = unsafe { window.vulkan_surface(vulkan.handle()) }?;
            unsafe { vk::raii::Surface::from_raw(vulkan.clone(), handle) }
        };

        let device = Device::new(&vulkan, &surface)?;
        let allocator = Allocator::new(device.clone())?;
        let transfer = TransferContext::new(device.clone(), allocator.clone())?;
        let swapchain = Swapchain::new(device.clone(), surface)?;
        let input = Input::new(window.sdl())?;
        let renderer = Renderer::new(device.clone(), &allocator, &swapchain)?;

        Ok(Self {
            renderer,
            swapchain,
            transfer,
            device,
            input,
            window,
        })
    }

    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    /// Uploads a CPU-side mesh to VRAM. The scene belongs to the caller; the
    /// engine only turns data into GPU resources.
    pub fn upload_mesh(&mut self, data: &MeshData) -> Result<Mesh> {
        Mesh::new(&mut self.transfer, data)
    }

    pub fn run(&mut self, mut run_info: RunInfo) -> Result<()> {
        let mut clock = Clock::new(Duration::from_hz(40));

        // +Z: the camera is right-handed and looks down -Z, so this is the
        // side from which the origin is visible.
        let mut camera = Camera::new(
            90.0,
            Vec3::new(0.0, 0.0, 3.0),
            0.0,
            0.0,
            aspect_ratio(self.swapchain.extent()),
        );

        loop {
            self.input.poll();
            if self.input.should_quit() {
                break;
            }

            clock.advance();

            // The context carries the camera in and out: the caller reads the
            // current state, writes the state it wants, and it is applied
            // before drawing.
            let mut frame_context = FrameContext {
                camera_pos: camera.position(),
                camera_yaw: camera.yaw(),
                camera_pitch: camera.pitch(),
                input: &self.input,
                timestep: clock.frame(),
            };

            (run_info.update)(&mut frame_context);

            camera.set_position(frame_context.camera_pos);
            camera.set_yaw(frame_context.camera_yaw);
            camera.set_pitch(frame_context.camera_pitch);
            camera.set_aspect(aspect_ratio(self.swapchain.extent()));

            self.draw(&camera, &run_info.scene)?;
        }

        self.device.wait_idle()?;
        println!("Exiting engine loop.");

        Ok(())
    }

    fn draw(&mut self, camera: &Camera, meshes: &[Mesh]) -> Result<()> {
        // Resolves the TODO left in Swapchain::Recreate in C++: minimized, the
        // surface has a 0x0 extent and recreating the swapchain would be
        // invalid.
        if self.input.minimized() {
            return Ok(());
        }

        // `frame` borrows the renderer; the swapchain is a disjoint field, so
        // it stays reachable to read the extent and to present on submit.
        let mut frame = self.renderer.begin_frame(&mut self.swapchain)?;
        frame.set_camera(camera);
        frame.draw_scene(meshes);
        frame.submit(&mut self.swapchain)?;

        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Runs before the fields are destroyed, so the device is still valid.
        // `.ok()` to avoid panicking inside a Drop.
        self.device.wait_idle().ok();
    }
}

/// Falls back to 1.0 while minimized: a 0x0 extent would make the projection
/// NaN, and the frame is skipped anyway.
fn aspect_ratio(extent: vk::Extent2D) -> f32 {
    if extent.height == 0 {
        return 1.0;
    }

    extent.width as f32 / extent.height as f32
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
        // Our own literal with no interior NUL byte: it could only fail if
        // someone edited it wrong.
        vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
    } else {
        Vec::new()
    };
    let layer_ptrs: Vec<*const c_char> = layers.iter().map(|name| name.as_ptr()).collect();

    // TODO (inherited from C++): check the supported extensions.
    let required_extensions = window.required_vulkan_extensions()?;
    let extension_ptrs: Vec<*const c_char> = required_extensions
        .iter()
        .map(|name| name.as_ptr())
        .collect();

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_ptrs)
        .enabled_extension_names(&extension_ptrs);

    // `LoadingError` is neither a Vulkan nor an SDL error, so it does not go
    // through `Context`: there is no VkResult at all, just the loader missing
    // from the machine.
    let entry = unsafe { vk::Entry::load() }
        .map_err(|error| Error::unsupported(format!("load the Vulkan loader: {error}")))?;

    let instance = unsafe { vk::raii::Instance::new(entry, &create_info) }
        .context("create the Vulkan instance")?;
    println!("Vulkan instance created.");

    Ok(instance)
}
