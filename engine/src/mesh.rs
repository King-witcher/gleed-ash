//! Equivalente a modules/engine/src/mesh.{h,cc}.

use ash::vk;

use crate::allocator::Buffer;
use crate::prelude::*;
use crate::transfer::TransferContext;
use crate::vertex::Vertex;

pub struct Mesh {
    // TODO (herdado do C++): tornar privado.
    pub vertex_count: u32,
    pub index_count: u32,

    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Mesh {
    pub fn new(transfer: &TransferContext, vertices: &[Vertex], indices: &[u32]) -> Result<Self> {
        Ok(Self {
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
            vertex_buffer: make_vertex_buffer(transfer, vertices)?,
            index_buffer: make_index_buffer(transfer, indices)?,
        })
    }

    pub fn vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &Buffer {
        &self.index_buffer
    }
}

fn make_vertex_buffer(transfer: &TransferContext, vertices: &[Vertex]) -> Result<Buffer> {
    // O vertex buffer mora na VRAM; o TransferContext cuida do staging.
    transfer.upload_slice(vertices, vk::BufferUsageFlags::VERTEX_BUFFER)
}

fn make_index_buffer(transfer: &TransferContext, indices: &[u32]) -> Result<Buffer> {
    // O index buffer mora na VRAM; o TransferContext cuida do staging.
    transfer.upload_slice(indices, vk::BufferUsageFlags::INDEX_BUFFER)
}
