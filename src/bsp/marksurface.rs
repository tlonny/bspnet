use std::io::{Error, ErrorKind, Result};

const MARKSURFACE_SIZE: usize = 2;

/// Decode leaf face references as unsigned little-endian face indices.
pub fn decode(bytes: &[u8]) -> Result<Vec<u16>> {
    if bytes.len() % MARKSURFACE_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "marksurfaces lump length must be a multiple of 2",
        ));
    }
    Ok(bytes
        .chunks_exact(MARKSURFACE_SIZE)
        .map(|entry| u16::from_le_bytes(entry.try_into().unwrap()))
        .collect())
}
