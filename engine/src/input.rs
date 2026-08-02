//! Equivalente a modules/engine/src/input.{h,cc}.
//!
//! Mesma máquina de estados do C++: `update()` drena a fila de eventos da SDL
//! e atualiza os conjuntos consultados por `is_key_down` / `was_key_pressed`.
//!
//! Diferença de representação: o C++ usava `bool keysDown[512]` indexado pelo
//! scancode; aqui `Scancode` é um enum, então usamos `HashSet`. Mesma semântica.

use std::collections::HashSet;

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Scancode;
use sdl3::mouse::MouseButton;
use sdl3::EventPump;

use crate::prelude::*;

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
            event_pump: sdl.event_pump().context("criar o event pump da SDL")?,
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

    pub fn update(&mut self) {
        // NOTA: igual ao C++, os conjuntos `*_pressed` NÃO são limpos aqui —
        // só em `clear()`. Ou seja, `was_key_pressed` continua verdadeiro até
        // alguém chamar `clear()`. Para virar "pressionado neste frame", bastava
        // limpar os dois no topo deste método. Mantido como está por fidelidade.
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

    pub fn clear(&mut self) {
        self.keys_down.clear();
        self.keys_pressed.clear();
        self.mouse_buttons_down.clear();
        self.mouse_buttons_pressed.clear();
    }
}
