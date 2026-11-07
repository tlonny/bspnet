use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

use super::contents::Contents;

const RECORD_SIZE: usize = 64;
const CLIP_HULL_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderRoot {
    Node(usize),
    Leaf(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipRoot {
    Clipnode(usize),
    Contents(Contents),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// Hull zero's root in the node or leaf lump.
    pub render_root: RenderRoot,
    /// Roots for collision hulls one through three, in file order.
    pub clip_roots: [ClipRoot; CLIP_HULL_COUNT],
    /// Visibility leaf count, excluding solid leaf zero.
    pub visleafs: usize,
    /// Range in the face lump.
    pub faces: Range<usize>,
}

impl Model {
    /// Validate the render leaf root and visibility count after leaf decoding.
    pub fn validate_leaves(&self, leaf_count: usize) -> Result<()> {
        if let RenderRoot::Leaf(index) = self.render_root {
            if index >= leaf_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "model references missing leaf",
                ));
            }
        }
        if self.visleafs > leaf_count.saturating_sub(1) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "model visibility leaf count exceeds leaf lump",
            ));
        }
        Ok(())
    }
}

// Unpack a collision root into a clipnode index or terminal contents.
fn clip_root(value: i32, clipnode_count: usize) -> Result<ClipRoot> {
    if value < 0 {
        return Ok(ClipRoot::Contents(Contents::decode(value)?));
    }
    let index = value as usize;
    if index >= clipnode_count {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "model references missing clipnode",
        ));
    }
    Ok(ClipRoot::Clipnode(index))
}

/// Decode models and validate their face ranges and node/clipnode roots.
/// Render leaf roots are checked separately after leaf decoding.
pub fn decode(
    bytes: &[u8],
    face_count: usize,
    node_count: usize,
    clipnode_count: usize,
) -> Result<Vec<Model>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "models lump length must be a multiple of 64",
        ));
    }
    records
        .iter()
        .map(|record| {
            let root = i32::from_le_bytes(record[36..40].try_into().unwrap());
            let render_root = if root < 0 {
                RenderRoot::Leaf((root.unsigned_abs() - 1) as usize)
            } else {
                if root as usize >= node_count {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "model references missing node",
                    ));
                }
                RenderRoot::Node(root as usize)
            };
            let clip_roots = [
                clip_root(
                    i32::from_le_bytes(record[40..44].try_into().unwrap()),
                    clipnode_count,
                )?,
                clip_root(
                    i32::from_le_bytes(record[44..48].try_into().unwrap()),
                    clipnode_count,
                )?,
                clip_root(
                    i32::from_le_bytes(record[48..52].try_into().unwrap()),
                    clipnode_count,
                )?,
            ];
            let visleafs = usize::try_from(i32::from_le_bytes(record[52..56].try_into().unwrap()))
                .map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidData,
                        "negative model visibility leaf count",
                    )
                })?;
            let start = usize::try_from(i32::from_le_bytes(record[56..60].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative model face offset"))?;
            let count = usize::try_from(i32::from_le_bytes(record[60..64].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative model face count"))?;
            let end = start
                .checked_add(count)
                .filter(|&end| end <= face_count)
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "model references missing faces")
                })?;
            Ok(Model {
                render_root,
                clip_roots,
                visleafs,
                faces: start..end,
            })
        })
        .collect()
}
