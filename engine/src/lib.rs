mod device;
mod engine;
pub mod error;
mod internal_prelude;
mod memory;
pub mod mesh;
mod platform;
mod renderer;
mod swapchain;
pub mod texture;
pub mod time;

pub use engine::{Engine, FrameContext, RunInfo};
pub use error::{Context, Error, IntoError, Result};
pub use glam;
pub use platform::input;

pub mod prelude {
    pub use crate::error::Context;
    pub use glam;
}
