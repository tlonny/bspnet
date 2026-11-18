use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

use super::contents::Contents;

const RECORD_SIZE: usize = 28;
const NO_VISIBILITY: i32 = -1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    pub contents: Contents,
    /// Range of visible leaf indices in the shared visibility lump, if present.
    pub pvs: Option<Range<usize>>,
    /// Range in the marksurface lump.
    pub marksurfaces: Range<usize>,
}

/// Decode render leaves and resolve PVS byte offsets using the transient row map.
pub fn decode(
    bytes: &[u8],
    visibility_ranges: &HashMap<usize, Range<usize>>,
    marksurface_count: usize,
) -> Result<Vec<Leaf>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "leaves lump length must be a multiple of 28",
        ));
    }
    records
        .iter()
        .map(|record| {
            let contents = Contents::decode(i32::from_le_bytes(record[..4].try_into().unwrap()))?;
            let offset = i32::from_le_bytes(record[4..8].try_into().unwrap());
            let pvs = if offset == NO_VISIBILITY {
                None
            } else {
                let offset = usize::try_from(offset).map_err(|_| {
                    Error::new(ErrorKind::InvalidData, "negative leaf visibility offset")
                })?;
                Some(
                    visibility_ranges
                        .get(&offset)
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                "leaf references missing visibility row",
                            )
                        })?
                        .clone(),
                )
            };
            let start = usize::from(u16::from_le_bytes(record[20..22].try_into().unwrap()));
            let count = usize::from(u16::from_le_bytes(record[22..24].try_into().unwrap()));
            let end = start
                .checked_add(count)
                .filter(|&end| end <= marksurface_count)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidData,
                        "leaf references missing marksurfaces",
                    )
                })?;
            Ok(Leaf {
                contents,
                pvs,
                marksurfaces: start..end,
            })
        })
        .collect()
}

#[cfg(test)]
mod test;
