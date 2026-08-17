use sdl3::event::{Event, WindowEvent};
use sdl3::EventPump;

use crate::internal_prelude::*;

pub use sdl3::keyboard::Scancode;
pub use sdl3::mouse::MouseButton;

#[derive(Clone, Copy, Default)]
pub struct MouseVector {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy)]
struct BitSet<const WORDS: usize>([u64; WORDS]);

impl<const WORDS: usize> BitSet<WORDS> {
    fn new() -> Self {
        Self([0; WORDS])
    }

    fn word_and_bit(index: usize) -> (usize, u64) {
        (index >> 6, 1 << (index & 0x3f))
    }

    fn insert(&mut self, index: usize) {
        let (word, bit) = Self::word_and_bit(index);
        self.0[word] |= bit;
    }

    fn remove(&mut self, index: usize) {
        let (word, bit) = Self::word_and_bit(index);
        self.0[word] &= !bit;
    }

    fn contains(&self, index: usize) -> bool {
        let (word, bit) = Self::word_and_bit(index);
        self.0[word] & bit != 0
    }

    fn clear(&mut self) {
        self.0 = [0; WORDS];
    }

    fn drain_into(&mut self, other: &mut Self) {
        for (own, other) in self.0.iter_mut().zip(&mut other.0) {
            *other |= *own;
            *own = 0;
        }
    }
}

type Keys = BitSet<{ (Scancode::Count as usize).div_ceil(64) }>;
type MouseButtons = BitSet<{ (MouseButton::X2 as usize).div_ceil(64) }>;

pub struct Input {
    event_pump: EventPump,

    keys_down: Keys,
    keys_pressed: Keys,
    keys_released: Keys,

    mouse_buttons_down: MouseButtons,
    mouse_buttons_pressed: MouseButtons,
    mouse_buttons_released: MouseButtons,

    minimized: bool,
    should_quit: bool,

    mouse_absolute: MouseVector,
    mouse_delta: MouseVector,
}

impl Input {
    pub fn is_key_down(&self, scancode: Scancode) -> bool {
        self.keys_down.contains(scancode as usize)
    }

    pub fn was_key_pressed(&self, scancode: Scancode) -> bool {
        self.keys_pressed.contains(scancode as usize)
    }

    pub fn was_key_released(&self, scancode: Scancode) -> bool {
        self.keys_released.contains(scancode as usize)
    }

    pub fn is_mouse_btn_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(button as usize)
    }

    pub fn was_mouse_btn_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(button as usize)
    }

    pub fn was_mouse_btn_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_released.contains(button as usize)
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

    fn clear(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_buttons_pressed.clear();
        self.mouse_buttons_released.clear();
        self.mouse_delta = MouseVector::default();
    }

    pub(crate) fn new(sdl: &sdl3::Sdl) -> Result<Self> {
        Ok(Self {
            event_pump: sdl.event_pump().context("create the SDL event pump")?,
            keys_down: Keys::new(),
            keys_pressed: Keys::new(),
            keys_released: Keys::new(),
            mouse_buttons_down: MouseButtons::new(),
            mouse_buttons_pressed: MouseButtons::new(),
            mouse_buttons_released: MouseButtons::new(),
            minimized: false,
            should_quit: false,
            mouse_absolute: MouseVector::default(),
            mouse_delta: MouseVector::default(),
        })
    }

    pub(crate) fn poll(&mut self) {
        self.clear();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::KeyDown {
                    scancode: Some(scancode),
                    repeat,
                    ..
                } => {
                    if !repeat {
                        self.keys_down.insert(scancode as usize);
                        self.keys_pressed.insert(scancode as usize);
                        if scancode == Scancode::Escape {
                            self.should_quit = true;
                        }
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    self.keys_down.remove(scancode as usize);
                    self.keys_released.insert(scancode as usize);
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    self.mouse_buttons_down.insert(mouse_btn as usize);
                    self.mouse_buttons_pressed.insert(mouse_btn as usize);
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    self.mouse_buttons_down.remove(mouse_btn as usize);
                    self.mouse_buttons_released.insert(mouse_btn as usize);
                }
                Event::Quit { .. } => {
                    self.should_quit = true;
                }
                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    self.mouse_absolute = MouseVector { x, y };
                    self.mouse_delta.x += xrel;
                    self.mouse_delta.y += yrel;
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Minimized => self.minimized = true,
                    WindowEvent::Restored => self.minimized = false,
                    WindowEvent::FocusLost => {
                        self.keys_down.drain_into(&mut self.keys_released);
                        self.mouse_buttons_down
                            .drain_into(&mut self.mouse_buttons_released);
                        self.mouse_delta = MouseVector::default();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
