use bytemuck::{Pod, Zeroable};
use glam::Mat4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Transforms {
    pub model: Mat4,
    pub camera: Mat4,
}
