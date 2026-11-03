use std::io::{Error, ErrorKind, Result};

const SURFEDGE_SIZE: usize = 4;

/// Decode a surfedges lump, preserving each edge index's direction sign.
pub fn decode(bytes: &[u8]) -> Result<Vec<i32>> {
    if bytes.len() % SURFEDGE_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "surfedges lump length must be a multiple of 4",
        ));
    }

    Ok(bytes
        .chunks_exact(SURFEDGE_SIZE)
        .map(|surfedge| i32::from_le_bytes(surfedge.try_into().unwrap()))
        .collect())
}
