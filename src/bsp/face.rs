use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

const RECORD_SIZE: usize = 20;
const LIGHT_STYLE_COUNT: usize = 4;
const UNUSED_LIGHT_STYLE: u8 = 255;
const NO_LIGHTING: i32 = -1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    /// Boundary range in the surfedge lump, preserving the file's winding.
    pub surfedges: Range<usize>,
    /// Index into the texinfo lump.
    pub texinfo: usize,
    /// Original light style slots; unused slots have no style.
    pub styles: [Option<u8>; LIGHT_STYLE_COUNT],
    /// Byte offset into the lighting lump, or no baked lighting.
    pub lighting_offset: Option<usize>,
}

/// Decode face records while leaving their references in their original lumps.
pub fn decode(
    bytes: &[u8],
    surfedge_count: usize,
    texinfo_count: usize,
    plane_count: usize,
) -> Result<Vec<Face>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "faces lump length must be a multiple of 20",
        ));
    }
    records
        .iter()
        .map(|record| {
            let plane = usize::try_from(i16::from_le_bytes(record[..2].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative face plane index"))?;
            if plane >= plane_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "face references missing plane",
                ));
            }
            let start = usize::try_from(i32::from_le_bytes(record[4..8].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative face surfedge offset"))?;
            let count = usize::try_from(i16::from_le_bytes(record[8..10].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative face surfedge count"))?;
            let end = start
                .checked_add(count)
                .filter(|&end| end <= surfedge_count)
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "face references missing surfedges")
                })?;
            let texinfo = usize::try_from(i16::from_le_bytes(record[10..12].try_into().unwrap()))
                .map_err(|_| {
                Error::new(ErrorKind::InvalidData, "negative face texinfo index")
            })?;
            if texinfo >= texinfo_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "face references missing texinfo",
                ));
            }
            let styles = std::array::from_fn(|slot| {
                let style = record[12 + slot];
                (style != UNUSED_LIGHT_STYLE).then_some(style)
            });
            let offset = i32::from_le_bytes(record[16..20].try_into().unwrap());
            let lighting_offset = if offset == NO_LIGHTING {
                None
            } else {
                Some(usize::try_from(offset).map_err(|_| {
                    Error::new(ErrorKind::InvalidData, "negative face lighting offset")
                })?)
            };
            Ok(Face {
                surfedges: start..end,
                texinfo,
                styles,
                lighting_offset,
            })
        })
        .collect()
}
