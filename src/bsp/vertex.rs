use std::io::{Error, ErrorKind, Result};

const RECORD_SIZE: usize = 12;
const COMPONENT_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Position in the file's original coordinates and map units.
    pub position: [f32; 3],
}

/// Decode positions without changing their coordinate system or order.
pub fn decode(bytes: &[u8]) -> Result<Vec<Vertex>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "vertices lump length must be a multiple of 12",
        ));
    }
    Ok(records
        .iter()
        .map(|record| Vertex {
            position: std::array::from_fn(|axis| {
                let start = axis * COMPONENT_SIZE;
                f32::from_le_bytes(record[start..start + COMPONENT_SIZE].try_into().unwrap())
            }),
        })
        .collect())
}
