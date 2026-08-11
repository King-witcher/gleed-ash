use glam::Mat4;

/// Transformações por frame enviadas ao shader. O layout precisa casar com o
/// `UniformBuffer` em shaders/mesh.slang (três float4x4 std140; o `Mat4` do
/// glam, como o do glm, já casa).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}
