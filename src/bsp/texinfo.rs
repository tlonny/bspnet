use std::io::{Error, ErrorKind, Result};

use glam::Vec4;

const TEXINFO_SIZE: usize = 40;

#[derive(Debug, PartialEq)]
pub struct Texinfo {
    /// S and T mapping rows: xyz is the axis, w is the pixel offset.
    pub axes: [Vec4; 2],
    /// Index into the BSP's textures array.
    pub texture: i32,
}

/// Decode texture mapping rows and texture indices, skipping the flags field.
pub fn decode(bytes: &[u8]) -> Result<Vec<Texinfo>> {
    if bytes.len() % TEXINFO_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "texinfo lump length must be a multiple of 40",
        ));
    }

    Ok(bytes
        .chunks_exact(TEXINFO_SIZE)
        .map(|info| Texinfo {
            axes: std::array::from_fn(|axis| {
                Vec4::from_array(std::array::from_fn(|component| {
                    let start = axis * 16 + component * 4;
                    f32::from_le_bytes(info[start..start + 4].try_into().unwrap())
                }))
            }),
            texture: i32::from_le_bytes(info[32..36].try_into().unwrap()),
        })
        .collect())
}
