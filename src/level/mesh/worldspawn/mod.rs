use std::io::{Error, ErrorKind, Result};

use crate::bsp::{Data as Bsp, texture::Kind};
use crate::gpu::{
    bind_group,
    buffer::{Index, vertex},
    render_pass,
};

use super::{batch, vertex as face_vertex};
use crate::level::{lightmap, texture};

struct Leaf {
    indices: Vec<u32>,
    batches: Vec<batch::Draw>,
}

pub struct Mesh {
    vertex_buffer: vertex::World,
    index_buffer: Index,
    leaves: Vec<Leaf>,
    current_leaf_id: Option<usize>,
}

impl Mesh {
    /// Upload worldspawn geometry and precompute each leaf's visible indices and batches.
    pub fn build(
        bsp: &Bsp,
        lightmap: &mut lightmap::Builder<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let mut world_vertices = Vec::new();
        let worldspawn = &bsp.models()[0];
        let mut face_indices = Vec::with_capacity(worldspawn.faces.len());
        for index in worldspawn.faces.clone() {
            let mapping = lightmap.build(bsp, index);
            let first = u32::try_from(world_vertices.len()).expect("too many world vertices");
            world_vertices.extend(face_vertex::build(bsp, index, &mapping));
            let end = u32::try_from(world_vertices.len()).expect("too many world vertices");
            // BSP faces wind clockwise; the render pipeline expects counterclockwise triangles.
            let indices = (first + 1..end.saturating_sub(1))
                .flat_map(|corner| [first, corner + 1, corner])
                .collect();
            face_indices.push(indices);
        }
        let vertex_buffer = vertex::World::create(device, world_vertices.len());
        vertex_buffer.write(queue, &world_vertices);
        let leaves: Vec<_> = (0..bsp.leaves().len())
            .map(|leaf_id| {
                let (indices, batches) = batch::build(bsp, &face_indices, leaf_id);
                Leaf { indices, batches }
            })
            .collect();
        let index_capacity = leaves
            .iter()
            .map(|leaf| leaf.indices.len())
            .max()
            .unwrap_or(0);

        Self {
            vertex_buffer,
            index_buffer: Index::create(device, index_capacity),
            leaves,
            current_leaf_id: None,
        }
    }

    /// Draw solids, sky, and liquids whose current opacity is fully opaque.
    pub fn draw_opaque<'a>(
        &'a self,
        pass: &mut render_pass::World<'a>,
        pipeline: &'a wgpu::RenderPipeline,
        camera: &'a bind_group::Camera,
        lightmaps: &'a bind_group::Array,
        textures: &'a [Option<texture::Texture>],
        sky_pipeline: &'a wgpu::RenderPipeline,
        liquid_pipeline: &'a wgpu::RenderPipeline,
        properties: &'a bind_group::Properties,
        water_alpha: f32,
        slime_properties: &'a bind_group::Properties,
        slime_alpha: f32,
    ) {
        let Some(leaf_id) = self.current_leaf_id else {
            return;
        };
        for batch in &self.leaves[leaf_id].batches {
            let texture = textures[batch.texture_id].as_ref().unwrap();
            let (liquid_properties, alpha) = if texture.kind() == Kind::Slime {
                (slime_properties, slime_alpha)
            } else {
                (properties, water_alpha)
            };
            let (pipeline, material, extra) = match texture.kind() {
                Kind::Solid | Kind::Portal => (pipeline, lightmaps.bind_group(), None),
                Kind::Sky => (sky_pipeline, properties.bind_group(), None),
                Kind::Water | Kind::Slime | Kind::Lava if alpha >= 1.0 => (
                    liquid_pipeline,
                    lightmaps.bind_group(),
                    Some(liquid_properties.bind_group()),
                ),
                Kind::Water | Kind::Slime | Kind::Lava => continue,
            };
            pass.draw(
                pipeline,
                camera,
                &self.vertex_buffer,
                &self.index_buffer,
                texture.binding(),
                material,
                extra,
                batch.indices.clone(),
            );
        }
    }

    /// Draw liquids with partial opacity; zero opacity contributes nothing.
    pub fn draw_transparent<'a>(
        &'a self,
        pass: &mut render_pass::World<'a>,
        pipeline: &'a wgpu::RenderPipeline,
        camera: &'a bind_group::Camera,
        lightmaps: &'a bind_group::Array,
        textures: &'a [Option<texture::Texture>],
        properties: &'a bind_group::Properties,
        water_alpha: f32,
        slime_properties: &'a bind_group::Properties,
        slime_alpha: f32,
    ) {
        let Some(leaf_id) = self.current_leaf_id else {
            return;
        };
        for batch in &self.leaves[leaf_id].batches {
            let texture = textures[batch.texture_id].as_ref().unwrap();
            let (properties, alpha) = match texture.kind() {
                Kind::Water | Kind::Lava => (properties, water_alpha),
                Kind::Slime => (slime_properties, slime_alpha),
                Kind::Solid | Kind::Portal | Kind::Sky => continue,
            };
            if alpha <= 0.0 || alpha >= 1.0 {
                continue;
            }
            pass.draw(
                pipeline,
                camera,
                &self.vertex_buffer,
                &self.index_buffer,
                texture.binding(),
                lightmaps.bind_group(),
                Some(properties.bind_group()),
                batch.indices.clone(),
            );
        }
    }

    /// Upload a leaf's precomputed indices, skipping an already selected leaf.
    pub fn set_leaf(&mut self, queue: &wgpu::Queue, leaf_id: usize) -> Result<bool> {
        let leaf = self
            .leaves
            .get(leaf_id)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing camera leaf"))?;
        if self.current_leaf_id == Some(leaf_id) {
            return Ok(false);
        }
        self.index_buffer.write(queue, &leaf.indices);
        self.current_leaf_id = Some(leaf_id);
        Ok(true)
    }
}

#[cfg(test)]
mod test;
