use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

use crate::{bsp, gpu::buffer::vertex::world};

mod mesh;
pub use mesh::Mesh;

// Resolve a directed edge to its starting vertex in the shared vertex array.
fn vertex_index(edges: &[[u16; 2]], vertex_count: usize, surfedge: i32) -> Result<u32> {
    if surfedge == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "surfedge references reserved edge 0",
        ));
    }
    let edge = edges
        .get(surfedge.unsigned_abs() as usize)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "surfedge references missing edge"))?;
    let index = edge[usize::from(surfedge < 0)];
    if usize::from(index) >= vertex_count {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "edge references missing vertex",
        ));
    }
    Ok(u32::from(index))
}

/// A contiguous group of triangle indices sharing a model and texture.
#[derive(Debug, PartialEq, Eq)]
pub struct TextureRange {
    pub model_id: usize,
    pub texture_id: usize,
    pub indices: Range<u32>,
}

/// CPU mesh buffers with draw ranges ordered by model, then texture.
pub struct MeshData {
    pub indices: Vec<u32>,
    pub vertices: Vec<world::Data>,
    pub ranges: Vec<TextureRange>,
    /// Triangle ranges indexed by original BSP face ID.
    pub face_indices: Vec<Range<u32>>,
}

/// Build indices grouped by model and texture, with BSP positions and texture UVs.
/// Vertices are shared where their BSP vertex and texinfo match. Clockwise face boundaries
/// become counterclockwise triangle fans for the world render pipeline.
pub fn triangulate(bsp: &bsp::Data) -> Result<MeshData> {
    let index_count = bsp.faces.iter().try_fold(0_u32, |total, face| {
        if face.num_surfedges < 3 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "face needs at least three corners",
            ));
        }
        total
            .checked_add(3 * (u32::from(face.num_surfedges) - 2))
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "too many triangle indices"))
    })?;
    let mut faces = Vec::with_capacity(bsp.faces.len());
    for (model_id, model) in bsp.models.iter().enumerate() {
        let start = usize::try_from(model.first_face)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "negative model face offset"))?;
        let count = usize::try_from(model.num_faces)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "negative model face count"))?;
        let model_faces = bsp
            .faces
            .get(start..)
            .and_then(|faces| faces.get(..count))
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "model references missing faces"))?;
        for (offset, face) in model_faces.iter().enumerate() {
            let info = bsp.texinfo.get(usize::from(face.texinfo)).ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "face references missing texinfo")
            })?;
            let texture_id = usize::try_from(info.texture)
                .ok()
                .filter(|&id| id < bsp.textures.len())
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "texinfo references missing texture")
                })?;
            faces.push((model_id, texture_id, start + offset, face));
        }
    }
    faces.sort_by_key(|&(model_id, texture_id, _, _)| (model_id, texture_id));
    let mut indices = Vec::with_capacity(index_count as usize);
    let mut vertices = Vec::new();
    let mut vertex_lookup = HashMap::new();
    let mut ranges: Vec<TextureRange> = Vec::new();
    let mut face_indices = vec![0..0; bsp.faces.len()];
    for (model_id, texture_id, face_id, face) in faces {
        let start = usize::try_from(face.first_surfedge)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "negative face surfedge offset"))?;
        let surfedges = bsp
            .surfedges
            .get(start..)
            .and_then(|remaining| remaining.get(..usize::from(face.num_surfedges)))
            .ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "face references missing surfedges")
            })?;
        let info = &bsp.texinfo[usize::from(face.texinfo)];
        let dimensions = bsp.textures[texture_id].dimensions;
        if dimensions.x == 0 || dimensions.y == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "zero texture dimensions",
            ));
        }
        let mut corners = Vec::with_capacity(surfedges.len());
        for &surfedge in surfedges {
            let source = vertex_index(&bsp.edges, bsp.vertices.len(), surfedge)?;
            let next_index = u32::try_from(vertices.len())
                .map_err(|_| Error::new(ErrorKind::InvalidData, "too many mesh vertices"))?;
            let index = *vertex_lookup
                .entry((source, face.texinfo))
                .or_insert_with(|| {
                    let position = bsp.vertices[source as usize];
                    let point = position.extend(1.0);
                    vertices.push(world::Data {
                        position,
                        uv: glam::Vec2::new(info.axes[0].dot(point), info.axes[1].dot(point))
                            / dimensions.as_vec2(),
                    });
                    next_index
                });
            corners.push(index);
        }
        let start = u32::try_from(indices.len())
            .map_err(|_| Error::new(ErrorKind::InvalidData, "too many triangle indices"))?;
        for pair in corners[1..].windows(2) {
            indices.extend([corners[0], pair[1], pair[0]]);
        }
        let end = u32::try_from(indices.len())
            .map_err(|_| Error::new(ErrorKind::InvalidData, "too many triangle indices"))?;
        face_indices[face_id] = start..end;
        if let Some(range) = ranges.last_mut()
            && range.model_id == model_id
            && range.texture_id == texture_id
        {
            range.indices.end = end;
        } else {
            ranges.push(TextureRange {
                model_id,
                texture_id,
                indices: start..end,
            });
        }
    }
    Ok(MeshData {
        indices,
        vertices,
        ranges,
        face_indices,
    })
}
