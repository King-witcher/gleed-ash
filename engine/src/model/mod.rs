mod vertex;

use std::rc::Rc;

use glam::{Mat4, Vec3};
pub use vertex::Vertex;

use crate::internal_prelude::*;
use crate::memory::{AllocatedBuffer, DeviceLocal, TransferContext};

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug)]
pub struct Mesh {
    index_count: u32,
    vertex_buffer: AllocatedBuffer<DeviceLocal>,
    index_buffer: AllocatedBuffer<DeviceLocal>,
}

pub struct Model {
    mesh: Rc<Mesh>,
    position: Vec3,
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

    pub fn vertex_buffer(&self) -> &AllocatedBuffer<DeviceLocal> {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &AllocatedBuffer<DeviceLocal> {
        &self.index_buffer
    }
}

impl Model {
    pub fn new(mesh: Rc<Mesh>, position: Vec3) -> Self {
        Self { mesh, position }
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, value: Vec3) {
        self.position = value
    }

    pub fn translate(&mut self, value: Vec3) {
        self.position += value;
    }

    pub(crate) fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
    }
}
