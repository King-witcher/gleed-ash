//! Equivalent to modules/engine/src/window.{h,cc}.
//!
//! Same SDL3 as the C++ engine, through the `sdl3` crate. With the "ash"
//! feature on, SDL already returns typed handles, and Cargo unifies those
//! bindings with the ones the `vk` crate reexports — so the types are the same
//! and there is no cast between SDL's `VkInstance` and ours.

use std::ffi::CString;

use sdl3::video::{Window as SdlWindow, WindowPos};
use sdl3::{Sdl, VideoSubsystem};

use crate::internal_prelude::*;

pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Window {
    // Field order IS destruction order in Rust. The window must die before the
    // video subsystem, and it before the SDL context.
    window: SdlWindow,
    _video: VideoSubsystem,
    sdl: Sdl,
}

impl Window {
    pub fn new(title: &str) -> Result<Self> {
        let sdl = sdl3::init().context("initialize SDL3")?;
        let video = sdl.video().context("initialize the SDL video subsystem")?;

        let window = video
            .window(title, 1920, 1080)
            .fullscreen()
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

    /// Handle to the SDL context — `Input` needs it to open the event pump.
    /// This did not exist in C++ because SDL_PollEvent is global.
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

        // A name with an interior NUL would be an SDL bug, not an environment
        // condition.
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
