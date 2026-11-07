use std::io::{Error, ErrorKind, Result};

const RECORD_SIZE: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Child {
    Node(usize),
    Leaf(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Index into the plane lump.
    pub plane: usize,
    pub front: Child,
    pub back: Child,
}

impl Node {
    /// Validate leaf children after the leaf lump has been decoded.
    pub fn validate_leaves(&self, leaf_count: usize) -> Result<()> {
        for child in [self.front, self.back] {
            if let Child::Leaf(index) = child {
                if index >= leaf_count {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "node references missing leaf",
                    ));
                }
            }
        }
        Ok(())
    }
}

// Unpack a child reference and validate any render-node index.
fn child(value: i16, node_count: usize) -> Result<Child> {
    if value < 0 {
        return Ok(Child::Leaf(usize::from(value.unsigned_abs() - 1)));
    }
    let index = value as usize;
    if index >= node_count {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "node references missing node",
        ));
    }
    Ok(Child::Node(index))
}

/// Decode render nodes and validate their node children and face ranges.
/// Leaf references are checked separately after leaf decoding.
pub fn decode(bytes: &[u8], face_count: usize, plane_count: usize) -> Result<Vec<Node>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "nodes lump length must be a multiple of 24",
        ));
    }
    records
        .iter()
        .map(|record| {
            let plane = usize::try_from(i32::from_le_bytes(record[..4].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative node plane index"))?;
            if plane >= plane_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "node references missing plane",
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
            let start = usize::from(u16::from_le_bytes(record[20..22].try_into().unwrap()));
            let count = usize::from(u16::from_le_bytes(record[22..24].try_into().unwrap()));
            start
                .checked_add(count)
                .filter(|&end| end <= face_count)
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "node references missing faces")
                })?;
            Ok(Node { plane, front, back })
        })
        .collect()
}
