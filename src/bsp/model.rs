use std::io::{Error, ErrorKind, Result};

const MODEL_SIZE: usize = 64;

#[derive(Debug)]
pub struct Model {
    /// Render-tree root, taken from headnode[0] in the file.
    pub headnode: i32,
    /// Number of visibility leaves, excluding solid leaf 0.
    pub visleafs: i32,
    /// Starting index in the BSP's faces array.
    pub first_face: i32,
    /// Number of consecutive faces belonging to this model.
    pub num_faces: i32,
}

/// Read the render-tree root, visibility leaf count, and face range.
pub fn decode(bytes: &[u8]) -> Result<Vec<Model>> {
    if bytes.len() % MODEL_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "models lump length must be a multiple of 64",
        ));
    }

    Ok(bytes
        .chunks_exact(MODEL_SIZE)
        .map(|model| Model {
            headnode: i32::from_le_bytes(model[36..40].try_into().unwrap()),
            visleafs: i32::from_le_bytes(model[52..56].try_into().unwrap()),
            first_face: i32::from_le_bytes(model[56..60].try_into().unwrap()),
            num_faces: i32::from_le_bytes(model[60..64].try_into().unwrap()),
        })
        .collect())
}
