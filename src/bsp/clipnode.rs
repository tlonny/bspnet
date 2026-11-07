use std::io::{Error, ErrorKind, Result};

use super::contents::Contents;

const RECORD_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Child {
    Clipnode(usize),
    Contents(Contents),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clipnode {
    /// Index into the plane lump.
    pub plane: usize,
    pub front: Child,
    pub back: Child,
}

// Unpack a child reference and validate any clipnode index.
fn child(value: i16, clipnode_count: usize) -> Result<Child> {
    if value < 0 {
        return Ok(Child::Contents(Contents::decode(i32::from(value))?));
    }
    let index = value as usize;
    if index >= clipnode_count {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "clipnode references missing clipnode",
        ));
    }
    Ok(Child::Clipnode(index))
}

/// Decode BSP29 clipnodes and validate their plane and clipnode references.
pub fn decode(bytes: &[u8], plane_count: usize) -> Result<Vec<Clipnode>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "clipnodes lump length must be a multiple of 8",
        ));
    }
    records
        .iter()
        .map(|record| {
            let plane = usize::try_from(i32::from_le_bytes(record[..4].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative clipnode plane index"))?;
            if plane >= plane_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "clipnode references missing plane",
                ));
            }
            let front = child(
                i16::from_le_bytes(record[4..6].try_into().unwrap()),
                records.len(),
            )?;
            let back = child(
                i16::from_le_bytes(record[6..8].try_into().unwrap()),
                records.len(),
            )?;
            Ok(Clipnode { plane, front, back })
        })
        .collect()
}
