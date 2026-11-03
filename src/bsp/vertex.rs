use std::io::{Error, ErrorKind, Result};

use glam::Vec3;

const VERTEX_SIZE: usize = 12;

/// Decode a vertices lump as little-endian positions in Quake coordinates.
pub fn decode(bytes: &[u8]) -> Result<Vec<Vec3>> {
    if bytes.len() % VERTEX_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "vertices lump length must be a multiple of 12",
        ));
    }

    Ok(bytes
        .chunks_exact(VERTEX_SIZE)
        .map(|vertex| {
            Vec3::from_array(std::array::from_fn(|axis| {
                let start = axis * 4;
                f32::from_le_bytes(vertex[start..start + 4].try_into().unwrap())
            }))
        })
        .collect())
}
