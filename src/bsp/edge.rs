use std::io::{Error, ErrorKind, Result};

const RECORD_SIZE: usize = 4;
const INDEX_SIZE: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// Ordered indices into the vertex lump.
    pub vertices: [usize; 2],
}

/// Decode and validate both vertex references of every edge.
pub fn decode(bytes: &[u8], vertex_count: usize) -> Result<Vec<Edge>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "edges lump length must be a multiple of 4",
        ));
    }
    records
        .iter()
        .map(|record| {
            let vertices = std::array::from_fn(|endpoint| {
                let start = endpoint * INDEX_SIZE;
                usize::from(u16::from_le_bytes(
                    record[start..start + INDEX_SIZE].try_into().unwrap(),
                ))
            });
            if vertices.iter().any(|&index| index >= vertex_count) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "edge references missing vertex",
                ));
            }
            Ok(Edge { vertices })
        })
        .collect()
}
