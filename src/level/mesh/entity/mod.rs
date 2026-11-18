use std::io::{Error, ErrorKind, Result};

use crate::bsp::Data as Bsp;
use crate::gpu::buffer::{Index, vertex as buffer_vertex};
use crate::level::lightmap;

pub use super::batch::Draw;
use super::{batch, vertex};

pub struct Mesh {
    vertices: buffer_vertex::World,
    indices: Index,
    batches: Vec<Draw>,
}

impl Mesh {
    /// Upload one brush model's complete geometry and pack its lighting into the shared atlas.
    pub fn build(
        bsp: &Bsp,
        model_index: usize,
        lightmap: &mut lightmap::Builder<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self> {
        let model = bsp
            .models()
            .get(model_index)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing brush model"))?;
        let mut data = Vec::new();
        let mut face_indices = Vec::with_capacity(model.faces.len());
        for face_id in model.faces.clone() {
            let mapping = lightmap.build(bsp, face_id);
            let first = u32::try_from(data.len()).expect("too many brush vertices");
            data.extend(vertex::build(bsp, face_id, &mapping));
            let end = u32::try_from(data.len()).expect("too many brush vertices");
            // BSP faces wind clockwise; the pipelines expect counterclockwise triangles.
            face_indices.push(
                (first + 1..end.saturating_sub(1))
                    .flat_map(|corner| [first, corner + 1, corner])
                    .collect(),
            );
        }
        let (triangles, batches) = batch::pack(
            bsp,
            &face_indices,
            model.faces.start,
            model.faces.clone().collect(),
        );
        let vertices = buffer_vertex::World::create(device, data.len());
        vertices.write(queue, &data);
        let mut indices = Index::create(device, triangles.len());
        indices.write(queue, &triangles);
        Ok(Self {
            vertices,
            indices,
            batches,
        })
    }

    pub fn vertices(&self) -> &buffer_vertex::World {
        &self.vertices
    }

    pub fn indices(&self) -> &Index {
        &self.indices
    }

    pub fn batches(&self) -> &[Draw] {
        &self.batches
    }
}

#[cfg(test)]
mod test;
