use std::io::{Error, ErrorKind, Result};

const RECORD_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Traverse from the edge's first vertex to its second.
    Forward,
    /// Traverse from the edge's second vertex to its first.
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surfedge {
    /// Index into the edge lump, with the sign encoding removed.
    pub edge: usize,
    pub direction: Direction,
}

/// Unpack signed edge references into indices and explicit directions.
pub fn decode(bytes: &[u8], edge_count: usize) -> Result<Vec<Surfedge>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "surfedges lump length must be a multiple of 4",
        ));
    }
    records
        .iter()
        .map(|record| {
            let signed = i32::from_le_bytes(*record);
            let edge = signed.unsigned_abs() as usize;
            if edge >= edge_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "surfedge references missing edge",
                ));
            }
            let direction = if signed < 0 {
                Direction::Reverse
            } else {
                Direction::Forward
            };
            Ok(Surfedge { edge, direction })
        })
        .collect()
}
