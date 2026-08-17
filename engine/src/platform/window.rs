use std::ffi::CString;

use sdl3::video::{Window as SdlWindow, WindowPos};
use sdl3::{Sdl, VideoSubsystem};

use crate::internal_prelude::*;

pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Window {
    window: SdlWindow,
    _video: VideoSubsystem,
    sdl: Sdl,
}

impl Window {
    pub fn new(title: &str) -> Result<Self> {
        let sdl = sdl3::init().context("initialize SDL3")?;
        let video = sdl.video().context("initialize the SDL video subsystem")?;

        let window = video
            .window(title, 600, 600)
            // .fullscreen()
            .vulkan()
            .resizable()
            .build()
            .context("create the window")?;

        Ok(Self {
            window,
            _video: video,
            sdl,
        })
    }

    pub fn sdl(&self) -> &Sdl {
        &self.sdl
    }

    /// # Safety
    /// `instance` must be a valid, live VkInstance.
    pub unsafe fn vulkan_surface(&self, instance: vk::Instance) -> Result<vk::SurfaceKHR> {
        self.window
            .vulkan_create_surface(instance)
            .context("create the Vulkan surface")
    }

    pub fn required_vulkan_extensions(&self) -> Result<Vec<CString>> {
        let names = self
            .window
            .vulkan_instance_extensions()
            .context("query the required Vulkan instance extensions")?;

        Ok(names
            .into_iter()
            .map(|name| CString::new(name).expect("extension name has an interior NUL"))
            .collect())
    }

    pub fn set_relative_mouse_mode(&mut self, on: bool) {
        self.sdl.mouse().set_relative_mouse_mode(&self.window, on);
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.window
            .set_position(WindowPos::Positioned(x), WindowPos::Positioned(y));
    }

    pub fn size(&self) -> Size {
        let (width, height) = self.window.size_in_pixels();
        Size { width, height }
    }
}
