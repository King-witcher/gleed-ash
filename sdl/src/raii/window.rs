use std::{ffi::CString, ops::Deref};

use sdl3::sys::video::{
    SDL_CreateWindow, SDL_DestroyWindow, SDL_SetWindowPosition, SDL_Window, SDL_WindowFlags,
};

use crate::{
    error::{Result, check, check_ptr},
    raii::Sdl,
};

pub struct Window {
    _sdl: Sdl,
    handle: *mut SDL_Window,
}

impl Window {
    pub fn create(sdl: Sdl, title: &str, w: i32, h: i32, flags: SDL_WindowFlags) -> Result<Self> {
        // An interior NUL in a title is a bug here, not an environment failure.
        let title = CString::new(title).expect("Null byte found in title");

        let handle = check_ptr("SDL_CreateWindow", unsafe {
            SDL_CreateWindow(title.as_ptr(), w, h, flags)
        })?;

        Ok(Self { _sdl: sdl, handle })
    }

    pub fn handle(&self) -> *mut SDL_Window {
        self.handle
    }

    pub fn set_position(&mut self, x: i32, y: i32) -> Result<()> {
        unsafe {
            check(
                "SDL_SetWindowPosition",
                SDL_SetWindowPosition(self.handle, x, y),
            )
        }
    }
}

impl Deref for Window {
    type Target = *mut SDL_Window;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            SDL_DestroyWindow(self.handle);
        }
    }
}
