//! Equivalent to modules/engine/src/vertex.h.
//!
//! TODO (inherited from C++): decouple from Vulkan.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

// repr(C) is required: the layout must match what the
// VertexInputAttributeDescriptions describe — without it Rust may reorder the
// fields. `Pod` also proves there is no padding, so a `&[Vertex]` can be
// reinterpreted as bytes for the upload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Vec3,
}

impl Vertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let position = vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(std::mem::offset_of!(Vertex, position) as u32);

        let color = vk::VertexInputAttributeDescription::default()
            .location(1)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(std::mem::offset_of!(Vertex, color) as u32);

        [position, color]
    }
}
