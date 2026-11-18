use std::io::{Error, ErrorKind, Result};

use super::texture::Texture;

const RECORD_SIZE: usize = 40;
const COMPONENT_SIZE: usize = 4;
const SPECIAL: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Texinfo {
    /// S and T projection rows: xyz is the axis, w is the pixel offset.
    pub axes: [[f32; 4]; 2],
    /// Validated index of a present entry in the texture lump.
    pub texture: usize,
}

/// Decode texture projections and validate their references to external texture headers.
pub fn decode(bytes: &[u8], textures: &[Option<Texture>]) -> Result<Vec<Texinfo>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "texinfo lump length must be a multiple of 40",
        ));
    }
    records
        .iter()
        .map(|record| {
            let axes = std::array::from_fn(|row| {
                std::array::from_fn(|component| {
                    let start = (row * 4 + component) * COMPONENT_SIZE;
                    f32::from_le_bytes(record[start..start + COMPONENT_SIZE].try_into().unwrap())
                })
            });
            let texture = usize::try_from(i32::from_le_bytes(record[32..36].try_into().unwrap()))
                .map_err(|_| {
                Error::new(ErrorKind::InvalidData, "negative texinfo texture index")
            })?;
            if textures.get(texture).and_then(Option::as_ref).is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "texinfo references missing texture",
                ));
            }
            let flags = i32::from_le_bytes(record[36..40].try_into().unwrap());
            if !matches!(flags, 0 | SPECIAL) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "unsupported texinfo flags",
                ));
            }
            Ok(Texinfo { axes, texture })
        })
        .collect()
}

#[cfg(test)]
mod test;
