//! Equivalent to modules/engine/src/input.{h,cc}.
//!
//! Same state machine as C++: `update()` drains the SDL event queue and
//! updates the sets queried by `is_key_down` / `was_key_pressed`.
//!
//! Representation difference: C++ used a `bool keysDown[512]` indexed by
//! scancode; here `Scancode` is an enum, so we use `HashSet`. Same semantics.

use std::collections::HashSet;

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Scancode;
use sdl3::mouse::MouseButton;
use sdl3::EventPump;

use crate::internal_prelude::*;

#[derive(Clone, Copy, Default)]
pub struct MouseVector {
    pub x: f32,
    pub y: f32,
}

pub struct Input {
    event_pump: EventPump,

    keys_down: HashSet<Scancode>,
    keys_pressed: HashSet<Scancode>,

    mouse_buttons_down: HashSet<MouseButton>,
    mouse_buttons_pressed: HashSet<MouseButton>,

    minimized: bool,
    should_quit: bool,

    mouse_absolute: MouseVector,
    mouse_delta: MouseVector,
}

impl Input {
    pub fn new(sdl: &sdl3::Sdl) -> Result<Self> {
        Ok(Self {
            event_pump: sdl.event_pump().context("create the SDL event pump")?,
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            mouse_buttons_down: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            minimized: false,
            should_quit: false,
            mouse_absolute: MouseVector::default(),
            mouse_delta: MouseVector::default(),
        })
    }

    pub(crate) fn poll(&mut self) {
        // NOTE: as in C++, the `*_pressed` sets are NOT cleared here — only in
        // `clear()`. That is, `was_key_pressed` stays true until someone calls
        // `clear()`. To mean "pressed this frame", clearing both at the top of
        // this method would do. Kept as is for fidelity.
        for event in self.event_pump.poll_iter() {
            match event {
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    self.keys_down.insert(scancode);
                    self.keys_pressed.insert(scancode);
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    self.keys_down.remove(&scancode);
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    self.mouse_buttons_down.insert(mouse_btn);
                    self.mouse_buttons_pressed.insert(mouse_btn);
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    self.mouse_buttons_down.remove(&mouse_btn);
                }
                Event::Quit { .. } => {
                    self.should_quit = true;
                }
                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    self.mouse_absolute = MouseVector { x, y };
                    self.mouse_delta = MouseVector { x: xrel, y: yrel };
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Minimized => self.minimized = true,
                    WindowEvent::Restored => self.minimized = false,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    pub fn is_key_down(&self, scancode: Scancode) -> bool {
        self.keys_down.contains(&scancode)
    }

    pub fn was_key_pressed(&self, scancode: Scancode) -> bool {
        self.keys_pressed.contains(&scancode)
    }

    pub fn is_mouse_btn_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    pub fn was_mouse_btn_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn minimized(&self) -> bool {
        self.minimized
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn mouse_absolute(&self) -> MouseVector {
        self.mouse_absolute
    }

    pub fn mouse_delta(&self) -> MouseVector {
        self.mouse_delta
    }

    // pub fn clear(&mut self) {
    //     self.keys_down.clear();
    //     self.keys_pressed.clear();
    //     self.mouse_buttons_down.clear();
    //     self.mouse_buttons_pressed.clear();
    // }
}
