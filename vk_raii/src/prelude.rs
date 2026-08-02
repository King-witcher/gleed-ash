pub use crate::*;
pub use ash::vk;
pub use std::rc::Rc;

pub type Result<T> = std::result::Result<T, vk::Result>;
