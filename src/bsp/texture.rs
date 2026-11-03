use std::io::{Error, ErrorKind, Result};

use glam::UVec2;

const INTEGER_SIZE: usize = 4;
const HEADER_SIZE: usize = 40;
const NAME_SIZE: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub name: String,
    pub dimensions: UVec2,
}

/// Decode texture names and dimensions in directory order, ignoring pixel data.
/// Supports header-only `-notex` entries and headers with embedded mip levels.
pub fn decode(bytes: &[u8]) -> Result<Vec<Texture>> {
    let count = bytes
        .get(..INTEGER_SIZE)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing texture count"))?;
    let count = usize::try_from(i32::from_le_bytes(count.try_into().unwrap()))
        .map_err(|_| Error::new(ErrorKind::InvalidData, "negative texture count"))?;
    let directory_end = count
        .checked_mul(INTEGER_SIZE)
        .and_then(|length| length.checked_add(INTEGER_SIZE))
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "texture directory too large"))?;
    let directory = bytes
        .get(INTEGER_SIZE..directory_end)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated texture directory"))?;

    directory
        .chunks_exact(INTEGER_SIZE)
        .map(|entry| {
            let offset = usize::try_from(i32::from_le_bytes(entry.try_into().unwrap()))
                .map_err(|_| Error::new(ErrorKind::InvalidData, "missing texture header"))?;
            if offset < directory_end {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "texture header overlaps directory",
                ));
            }
            let header = bytes
                .get(offset..)
                .and_then(|remaining| remaining.get(..HEADER_SIZE))
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "texture header outside lump"))?;
            let name_end = header[..NAME_SIZE]
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(NAME_SIZE);
            let name = std::str::from_utf8(&header[..name_end])
                .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid texture name"))?
                .to_owned();
            let dimensions = UVec2::new(
                u32::from_le_bytes(header[16..20].try_into().unwrap()),
                u32::from_le_bytes(header[20..24].try_into().unwrap()),
            );
            if dimensions.x == 0 || dimensions.y == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "zero texture dimensions",
                ));
            }
            Ok(Texture { name, dimensions })
        })
        .collect()
}
