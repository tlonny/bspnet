use std::io::{Error, ErrorKind, Result};

const INTEGER_SIZE: usize = 4;
const NAME_SIZE: usize = 16;
const HEADER_SIZE: usize = 40;
const MIP_COUNT: usize = 4;
const MISSING_TEXTURE: i32 = -1;

const SKY_PREFIX: &str = "sky";
const LIQUID_PREFIX: char = '*';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Solid,
    Sky,
    Liquid,
}

impl Kind {
    /// Classify sky and liquid prefixes, treating other textures as solid.
    fn from_name(name: &str) -> Self {
        if name
            .get(..SKY_PREFIX.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SKY_PREFIX))
        {
            Self::Sky
        } else if name.starts_with(LIQUID_PREFIX) {
            Self::Liquid
        } else {
            Self::Solid
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub name: String,
    pub kind: Kind,
    pub dimensions: [u32; 2],
    /// Palette indices for each mip level, in row-major order.
    pub mips: [Vec<u8>; MIP_COUNT],
}

/// Decode embedded BSP29 textures, keeping pixels as palette indices.
/// Missing directory entries remain None so texture indices retain their original meaning.
pub fn decode(bytes: &[u8]) -> Result<Vec<Option<Texture>>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let count = bytes
        .get(..INTEGER_SIZE)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated texture count"))?;
    let count = usize::try_from(i32::from_le_bytes(count.try_into().unwrap()))
        .map_err(|_| Error::new(ErrorKind::InvalidData, "negative texture count"))?;
    let table_end = count
        .checked_mul(INTEGER_SIZE)
        .and_then(|size| INTEGER_SIZE.checked_add(size))
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "texture directory too large"))?;
    let entries = bytes
        .get(INTEGER_SIZE..table_end)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated texture directory"))?;
    let offsets: Vec<_> = entries
        .chunks_exact(INTEGER_SIZE)
        .map(|entry| {
            let offset = i32::from_le_bytes(entry.try_into().unwrap());
            if offset == MISSING_TEXTURE {
                return Ok(None);
            }
            let offset = usize::try_from(offset)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "negative texture offset"))?;
            if offset < table_end || offset >= bytes.len() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "texture offset outside payload",
                ));
            }
            Ok(Some(offset))
        })
        .collect::<Result<_>>()?;
    offsets
        .iter()
        .map(|offset| {
            let Some(offset) = *offset else {
                return Ok(None);
            };
            let end = offsets
                .iter()
                .flatten()
                .copied()
                .filter(|&next| next > offset)
                .min()
                .unwrap_or(bytes.len());
            let record = &bytes[offset..end];
            let header = record
                .get(..HEADER_SIZE)
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated texture header"))?;
            let name = header[..NAME_SIZE].split(|&byte| byte == 0).next().unwrap();
            let name = std::str::from_utf8(name)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid texture name"))?;
            if name.is_empty() {
                return Err(Error::new(ErrorKind::InvalidData, "empty texture name"));
            }
            let dimensions = [
                u32::from_le_bytes(header[16..20].try_into().unwrap()),
                u32::from_le_bytes(header[20..24].try_into().unwrap()),
            ];
            if dimensions.contains(&0) {
                return Err(Error::new(ErrorKind::InvalidData, "zero texture dimension"));
            }
            let mut mips = std::array::from_fn(|_| Vec::new());
            for (level, mip) in mips.iter_mut().enumerate() {
                let field = 24 + level * INTEGER_SIZE;
                let start =
                    u32::from_le_bytes(header[field..field + INTEGER_SIZE].try_into().unwrap())
                        as usize;
                if start < HEADER_SIZE {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "expected embedded texture mip data",
                    ));
                }
                let count = ((dimensions[0] >> level).max(1) as usize)
                    .checked_mul((dimensions[1] >> level).max(1) as usize)
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData, "texture mip too large"))?;
                let pixels = record
                    .get(start..)
                    .and_then(|tail| tail.get(..count))
                    .ok_or_else(|| {
                        Error::new(ErrorKind::InvalidData, "texture mip outside texture record")
                    })?;
                *mip = pixels.to_vec();
            }
            Ok(Some(Texture {
                name: name.to_owned(),
                kind: Kind::from_name(name),
                dimensions,
                mips,
            }))
        })
        .collect()
}
