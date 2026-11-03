use std::io::{Error, ErrorKind, Result};

use glam::Vec3;

const PLANE_SIZE: usize = 20;

#[derive(Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

/// Decode splitting planes, ignoring the axial optimization hint.
pub fn decode(bytes: &[u8]) -> Result<Vec<Plane>> {
    if bytes.len() % PLANE_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "planes lump length must be a multiple of 20",
        ));
    }
    Ok(bytes
        .chunks_exact(PLANE_SIZE)
        .map(|plane| Plane {
            normal: Vec3::from_array(std::array::from_fn(|axis| {
                let start = axis * 4;
                f32::from_le_bytes(plane[start..start + 4].try_into().unwrap())
            })),
            distance: f32::from_le_bytes(plane[12..16].try_into().unwrap()),
        })
        .collect())
}
