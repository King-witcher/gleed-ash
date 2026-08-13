//! The geometry data model, from the single vertex up to the buffers in VRAM.
//! Equivalent to modules/engine/src/mesh.{h,cc} + vertex.h.

pub mod geometry;
mod vertex;

pub use vertex::Vertex;

use crate::memory::{Buffer, DeviceLocal, TransferContext};
use crate::prelude::*;

/// The CPU side of a mesh: plain data, no GPU resource involved. Produced by
/// the pure generators in [`geometry`] and consumed by [`Mesh::new`].
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// The GPU side: vertex and index buffers living in VRAM.
pub struct Mesh {
    index_count: u32,
    vertex_buffer: Buffer<DeviceLocal>,
    index_buffer: Buffer<DeviceLocal>,
}

impl Mesh {
    pub fn new(transfer: &mut TransferContext, data: &MeshData) -> Result<Self> {
        Ok(Self {
            index_count: data.indices.len() as u32,
            vertex_buffer: transfer
                .upload_slice(&data.vertices, vk::BufferUsageFlags::VERTEX_BUFFER)?,
            index_buffer: transfer
                .upload_slice(&data.indices, vk::BufferUsageFlags::INDEX_BUFFER)?,
        })
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn vertex_buffer(&self) -> &Buffer<DeviceLocal> {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &Buffer<DeviceLocal> {
        &self.index_buffer
    }
}
