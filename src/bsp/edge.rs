use std::io::{Error, ErrorKind, Result};

const EDGE_SIZE: usize = 4;

/// Decode an edges lump as pairs of little-endian vertex indices.
pub fn decode(bytes: &[u8]) -> Result<Vec<[u16; 2]>> {
    if bytes.len() % EDGE_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "edges lump length must be a multiple of 4",
        ));
    }

    Ok(bytes
        .chunks_exact(EDGE_SIZE)
        .map(|edge| {
            [
                u16::from_le_bytes(edge[..2].try_into().unwrap()),
                u16::from_le_bytes(edge[2..].try_into().unwrap()),
            ]
        })
        .collect())
}
