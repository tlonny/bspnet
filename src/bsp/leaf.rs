use std::io::{Error, ErrorKind, Result};

const LEAF_SIZE: usize = 28;

#[derive(Clone, Debug)]
pub struct Leaf {
    /// Quake contents code; leaf 0 represents solid space.
    pub contents: i32,
    /// Byte offset into the visibility lump; -1 means no visibility information.
    pub visibility_offset: i32,
    pub first_marksurface: u16,
    pub num_marksurfaces: u16,
}

/// Decode leaf contents, visibility offsets, and marksurface ranges.
pub fn decode(bytes: &[u8]) -> Result<Vec<Leaf>> {
    if bytes.len() % LEAF_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "leaves lump length must be a multiple of 28",
        ));
    }
    Ok(bytes
        .chunks_exact(LEAF_SIZE)
        .map(|leaf| Leaf {
            contents: i32::from_le_bytes(leaf[..4].try_into().unwrap()),
            visibility_offset: i32::from_le_bytes(leaf[4..8].try_into().unwrap()),
            first_marksurface: u16::from_le_bytes(leaf[20..22].try_into().unwrap()),
            num_marksurfaces: u16::from_le_bytes(leaf[22..24].try_into().unwrap()),
        })
        .collect())
}
