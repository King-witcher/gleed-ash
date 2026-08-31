use bytemuck::{Pod, Zeroable};
use glam::Mat4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub camera: Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PushConstants {
    pub model: Mat4,
}
