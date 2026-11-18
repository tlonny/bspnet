use crate::bsp::Data as Bsp;
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub struct Draw {
    pub texture_id: usize,
    pub indices: Range<u32>,
}

/// Sort selected faces by texture and concatenate their model-local triangles into draw ranges.
pub fn pack(
    bsp: &Bsp,
    face_indices: &[Vec<u32>],
    first_face: usize,
    mut visible: Vec<usize>,
) -> (Vec<u32>, Vec<Draw>) {
    visible.sort_unstable_by_key(|&index| bsp.texinfo()[bsp.faces()[index].texinfo].texture);
    let mut indices = Vec::new();
    let mut batches: Vec<Draw> = Vec::new();
    for index in visible {
        let face = &face_indices[index - first_face];
        let texture_id = bsp.texinfo()[bsp.faces()[index].texinfo].texture;
        if face.is_empty() {
            continue;
        }
        let start = u32::try_from(indices.len()).expect("too many visible indices");
        indices.extend_from_slice(face);
        let end = u32::try_from(indices.len()).expect("too many visible indices");
        if let Some(batch) = batches.last_mut() {
            if batch.texture_id == texture_id {
                batch.indices.end = end;
                continue;
            }
        }
        batches.push(Draw {
            texture_id,
            indices: start..end,
        });
    }
    (indices, batches)
}

/// Deduplicate visible faces and group their triangles into texture ranges.
pub fn build(bsp: &Bsp, face_indices: &[Vec<u32>], leaf_id: usize) -> (Vec<u32>, Vec<Draw>) {
    let world = &bsp.models()[0];
    let leaf = &bsp.leaves()[leaf_id];
    let mut seen = vec![false; face_indices.len()];
    let mut visible = Vec::new();
    if let Some(pvs) = &leaf.pvs {
        for &index in std::iter::once(&leaf_id).chain(bsp.visibility()[pvs.clone()].iter()) {
            for &face_id in &bsp.marksurfaces()[bsp.leaves()[index].marksurfaces.clone()] {
                if world.faces.contains(&face_id) && !seen[face_id - world.faces.start] {
                    seen[face_id - world.faces.start] = true;
                    visible.push(face_id);
                }
            }
        }
    } else {
        visible.extend(world.faces.clone());
    }
    pack(bsp, face_indices, world.faces.start, visible)
}
