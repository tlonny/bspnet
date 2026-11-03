use std::io::{Error, ErrorKind, Result};

const NODE_SIZE: usize = 24;

#[derive(Debug)]
pub struct Node {
    pub plane: i32,
    /// Front and back children; a negative child encodes leaf index -(child + 1).
    pub children: [i16; 2],
}

/// Decode render-tree branches from full BSP29 node records.
pub fn decode(bytes: &[u8]) -> Result<Vec<Node>> {
    if bytes.len() % NODE_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "nodes lump length must be a multiple of 24",
        ));
    }
    Ok(bytes
        .chunks_exact(NODE_SIZE)
        .map(|node| Node {
            plane: i32::from_le_bytes(node[..4].try_into().unwrap()),
            children: [
                i16::from_le_bytes(node[4..6].try_into().unwrap()),
                i16::from_le_bytes(node[6..8].try_into().unwrap()),
            ],
        })
        .collect())
}
