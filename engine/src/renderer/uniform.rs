use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// Per-frame transforms sent to the shader. The layout must match the
/// `UniformBuffer` in shaders/mesh.slang (three float4x4, std140; glam's
/// `Mat4`, like glm's, already does). `Pod` is what lets it be uploaded as
/// plain bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}
