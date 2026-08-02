//! Equivalente a modules/engine/src/window.{h,cc}.
//!
//! Mesma SDL3 do engine C++, só que pela crate `sdl3`. Com a feature "ash"
//! ligada, o SDL já devolve os handles tipados como `ash::vk::*`, então não
//! existe nenhum cast entre o `VkInstance` do SDL e o do ash.

use std::ffi::CString;

use ash::vk;
use sdl3::video::{Window as SdlWindow, WindowFlags, WindowPos};
use sdl3::{Sdl, VideoSubsystem};

use crate::prelude::*;

pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Window {
    // A ordem dos campos É a ordem de destruição no Rust. A janela precisa
    // morrer antes do subsistema de vídeo, e este antes do contexto SDL.
    window: SdlWindow,
    _video: VideoSubsystem,
    sdl: Sdl,
}

impl Window {
    pub fn new(title: &str) -> Result<Self> {
        let sdl = sdl3::init().context("inicializar a SDL3")?;
        let video = sdl
            .video()
            .context("inicializar o subsistema de vídeo da SDL")?;

        // set_flags() sobrescreve os flags acumulados, então vem primeiro;
        // vulkan()/resizable() depois só acrescentam. Mesma combinação do C++:
        // SDL_WINDOW_VULKAN | SDL_WINDOW_MOUSE_RELATIVE_MODE | SDL_WINDOW_RESIZABLE.
        let window = video
            .window(title, 800, 600)
            .set_flags(WindowFlags::MOUSE_RELATIVE_MODE)
            .vulkan()
            .resizable()
            .build()
            .context("criar a janela")?;

        Ok(Self {
            window,
            _video: video,
            sdl,
        })
    }

    /// Handle do contexto SDL — o `Input` precisa dele para abrir o event pump.
    /// No C++ isto não existia porque SDL_PollEvent é global.
    pub fn sdl(&self) -> &Sdl {
        &self.sdl
    }

    /// # Safety
    /// `instance` precisa ser uma VkInstance válida e viva.
    pub unsafe fn vulkan_surface(&self, instance: vk::Instance) -> Result<vk::SurfaceKHR> {
        self.window
            .vulkan_create_surface(instance)
            .context("criar a surface Vulkan")
    }

    /// Extensões de instância que a SDL exige para conseguir criar a surface.
    /// O C++ devolvia um `span<const char*>` apontando para memória da SDL;
    /// aqui copiamos para `CString` para não depender do tempo de vida dela.
    pub fn required_vulkan_extensions(&self) -> Result<Vec<CString>> {
        let names = self
            .window
            .vulkan_instance_extensions()
            .context("consultar as extensões de instância Vulkan exigidas")?;

        // Um nome com NUL no meio seria bug da SDL, não condição de ambiente.
        Ok(names
            .into_iter()
            .map(|name| CString::new(name).expect("extension name has an interior NUL"))
            .collect())
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.window
            .set_position(WindowPos::Positioned(x), WindowPos::Positioned(y));
    }

    pub fn get_size(&self) -> Size {
        let (width, height) = self.window.size_in_pixels();
        Size { width, height }
    }
}
