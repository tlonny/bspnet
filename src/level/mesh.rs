use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

use crate::{
    bsp,
    gpu::{
        bind_group,
        buffer::{Index, vertex},
        pipeline, render_pass,
    },
};

use super::{TextureRange, triangulate};

#[cfg(test)]
#[path = "test.rs"]
mod test;

struct Face {
    texture_id: usize,
    indices: Range<u32>,
}

// Own the visibility inputs and reusable CPU storage for a packed world draw.
struct Visibility {
    faces: Vec<Face>,
    leaves: Vec<bsp::Leaf>,
    visibility: Vec<u8>,
    marksurfaces: Vec<u16>,
    world_faces: Range<usize>,
    visleafs: usize,
    source_indices: Vec<u32>,
    seen: Vec<bool>,
    selected: Vec<usize>,
    packed: Vec<u32>,
    ranges: Vec<TextureRange>,
}

impl Visibility {
    // Validate and copy the world visibility data without retaining the BSP.
    fn build(
        bsp: &bsp::Data,
        source_indices: Vec<u32>,
        face_indices: Vec<Range<u32>>,
    ) -> Result<Self> {
        let world = bsp
            .models
            .first()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing world model"))?;
        let start = usize::try_from(world.first_face)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "negative world face offset"))?;
        let count = usize::try_from(world.num_faces)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "negative world face count"))?;
        let world_faces = start..start + count;
        let visleafs = usize::try_from(world.visleafs)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "negative visibility leaf count"))?;
        if visleafs >= bsp.leaves.len() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "visibility references missing leaves",
            ));
        }
        for leaf in &bsp.leaves {
            let start = usize::from(leaf.first_marksurface);
            let end = start + usize::from(leaf.num_marksurfaces);
            let marks = bsp.marksurfaces.get(start..end).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    "leaf references missing marksurfaces",
                )
            })?;
            if marks.iter().any(|&id| usize::from(id) >= bsp.faces.len()) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "marksurface references missing face",
                ));
            }
        }
        let faces = bsp
            .faces
            .iter()
            .zip(face_indices)
            .map(|(face, indices)| {
                let info = bsp.texinfo.get(usize::from(face.texinfo)).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "face references missing texinfo")
                })?;
                let texture_id = usize::try_from(info.texture)
                    .ok()
                    .filter(|&id| id < bsp.textures.len())
                    .ok_or_else(|| {
                        Error::new(ErrorKind::InvalidData, "face references missing texture")
                    })?;
                Ok(Face {
                    texture_id,
                    indices,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let capacity = source_indices.len();
        Ok(Self {
            faces,
            leaves: bsp.leaves.clone(),
            visibility: bsp.visibility.clone(),
            marksurfaces: bsp.marksurfaces.clone(),
            world_faces,
            visleafs,
            source_indices,
            seen: vec![false; bsp.faces.len()],
            selected: Vec::with_capacity(count),
            packed: Vec::with_capacity(capacity),
            ranges: Vec::with_capacity(bsp.textures.len()),
        })
    }

    // Append each face referenced by a visible leaf at most once.
    fn collect_leaf(&mut self, id: usize) {
        let leaf = &self.leaves[id];
        let start = usize::from(leaf.first_marksurface);
        let end = start + usize::from(leaf.num_marksurfaces);
        for &id in &self.marksurfaces[start..end] {
            let id = usize::from(id);
            if self.world_faces.contains(&id) && !self.seen[id] {
                self.seen[id] = true;
                self.selected.push(id);
            }
        }
    }

    // Decode BSP29 visibility bits and pack deduplicated faces in texture order.
    fn select(&mut self, leaf_id: usize) -> Result<()> {
        let leaf = self
            .leaves
            .get(leaf_id)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing target leaf"))?;
        let offset = leaf.visibility_offset;
        self.seen.fill(false);
        self.selected.clear();
        self.packed.clear();
        self.ranges.clear();
        if leaf_id == 0 || offset == -1 {
            self.selected.extend(self.world_faces.clone());
        } else {
            let mut cursor = usize::try_from(offset)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative visibility offset"))?;
            let row_bytes = self.visleafs.div_ceil(8);
            let mut byte_index = 0;
            while byte_index < row_bytes {
                let bits = *self.visibility.get(cursor).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "truncated visibility row")
                })?;
                cursor += 1;
                if bits == 0 {
                    let run = usize::from(*self.visibility.get(cursor).ok_or_else(|| {
                        Error::new(ErrorKind::InvalidData, "truncated visibility run")
                    })?);
                    cursor += 1;
                    if run == 0 || run > row_bytes - byte_index {
                        return Err(Error::new(ErrorKind::InvalidData, "invalid visibility run"));
                    }
                    byte_index += run;
                } else {
                    for bit in 0..8 {
                        let index = byte_index * 8 + bit;
                        // PVS bit zero describes leaf 1; solid leaf 0 has no bit.
                        if index < self.visleafs && bits & (1 << bit) != 0 {
                            self.collect_leaf(index + 1);
                        }
                    }
                    byte_index += 1;
                }
            }
            // Keep the camera's own leaf even if the source row omits its bit.
            self.collect_leaf(leaf_id);
        }
        self.selected
            .sort_unstable_by_key(|&id| (self.faces[id].texture_id, id));
        for &id in &self.selected {
            let face = &self.faces[id];
            let start = self.packed.len() as u32;
            self.packed.extend_from_slice(
                &self.source_indices[face.indices.start as usize..face.indices.end as usize],
            );
            let end = self.packed.len() as u32;
            if start == end {
                continue;
            }
            if let Some(range) = self.ranges.last_mut()
                && range.texture_id == face.texture_id
            {
                range.indices.end = end;
            } else {
                self.ranges.push(TextureRange {
                    model_id: 0,
                    texture_id: face.texture_id,
                    indices: start..end,
                });
            }
        }
        Ok(())
    }
}

/// Owned world geometry with static vertices and leaf-dependent triangle indices.
pub struct Mesh {
    textures: Vec<bsp::Texture>,
    visibility: Visibility,
    vertices: vertex::World,
    indices: Index,
    cached_leaf: Option<usize>,
}

impl Mesh {
    /// Copy rendering data from the BSP and upload its fixed vertex geometry once.
    pub fn build(device: &wgpu::Device, queue: &wgpu::Queue, bsp: &bsp::Data) -> Result<Self> {
        let data = triangulate(bsp)?;
        let visibility = Visibility::build(bsp, data.indices, data.face_indices)?;
        let vertices = vertex::World::create(device, data.vertices.len());
        vertices.write(queue, &data.vertices);
        let indices = Index::create(device, visibility.source_indices.len());
        Ok(Self {
            textures: bsp.textures.clone(),
            visibility,
            vertices,
            indices,
            cached_leaf: None,
        })
    }

    /// Texture metadata in BSP directory order, matching render_leaf's bindings.
    pub fn textures(&self) -> &[bsp::Texture] {
        &self.textures
    }

    /// Draw a leaf's PVS, rebuilding and uploading indices only when the leaf changes.
    /// Submit this pass before rendering another leaf with the same mesh buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn render_leaf<'a>(
        &'a mut self,
        queue: &wgpu::Queue,
        pass: &mut render_pass::World<'a>,
        pipeline: &'a pipeline::World,
        camera: &'a bind_group::Camera,
        textures: &'a [bind_group::Color],
        leaf_id: usize,
    ) -> Result<()> {
        if textures.len() != self.textures.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "texture bindings must match mesh metadata",
            ));
        }
        if self.cached_leaf != Some(leaf_id) {
            // A failed selection must not leave the old cache marked valid.
            self.cached_leaf = None;
            self.visibility.select(leaf_id)?;
            self.indices.write(queue, &self.visibility.packed);
            self.cached_leaf = Some(leaf_id);
        }
        pass.draw(
            pipeline,
            camera,
            &self.vertices,
            &self.indices,
            textures,
            &self.visibility.ranges,
        );
        Ok(())
    }
}
