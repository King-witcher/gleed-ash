pub mod raii;

pub mod dispatch {
    pub use ash::{Device, Instance};
}

pub use ash::prelude::VkResult;
pub use ash::vk::*;
pub use ash::{Entry, khr, util};
