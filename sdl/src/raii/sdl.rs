use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use sdl3::sys::{
    init::{SDL_Init, SDL_InitFlags, SDL_Quit},
    video::SDL_WindowFlags,
};

use crate::{
    error::{Error, Result, check},
    raii::Window,
};

struct SdlInner;

impl Drop for SdlInner {
    fn drop(&mut self) {
        unsafe { SDL_Quit() };
        INITIALIZED.store(false, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct Sdl(Rc<SdlInner>);

// AI Generated: I don't fully understand atomic operations and ask Claude Code to do that for me.
// This is one of the few parts of this codebase that aren't mine.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl Sdl {
    pub fn init(flags: SDL_InitFlags) -> Result<Self> {
        if INITIALIZED.swap(true, Ordering::Acquire) {
            return Err(Error::with_message(
                "SDL_Init",
                "SDL is already initialized",
            ));
        }

        if let Err(error) = check("SDL_Init", unsafe { SDL_Init(flags) }) {
            INITIALIZED.store(false, Ordering::Release);
            return Err(error);
        }

        Ok(Self(Rc::new(SdlInner)))
    }

    pub fn create_window(
        &self,
        title: &str,
        w: i32,
        h: i32,
        flags: SDL_WindowFlags,
    ) -> Result<Window> {
        Window::create(self.clone(), title, w, h, flags)
    }
}
