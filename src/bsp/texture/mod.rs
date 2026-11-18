use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;

const INTEGER_SIZE: usize = 4;
const NAME_SIZE: usize = 16;
const HEADER_SIZE: usize = 40;
const MIP_COUNT: usize = 4;
const MISSING_TEXTURE: i32 = -1;

const SKY_PREFIX: &str = "sky";
const PORTAL_PREFIX: &str = "portal";
const SLIME_PREFIX: &str = "*slime";
const LAVA_PREFIX: &str = "*lava";
const LIQUID_PREFIX: char = '*';
const FRAME_PREFIX: u8 = b'+';
const FIRST_FRAME: u8 = 0;

#[derive(Debug, PartialEq, Eq)]
pub struct Family {
    /// Texture IDs in numbered frame order, or the first alternate as a fallback.
    pub normal: Vec<usize>,
    /// Texture IDs in lettered frame order, when present.
    pub alternate: Option<Vec<usize>>,
}

/// Maps each member's original texture ID to its shared family.
pub type Families = HashMap<usize, Arc<Family>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Solid,
    Portal,
    Sky,
    Water,
    Slime,
    Lava,
}

impl Kind {
    /// Identify surfaces rendered by the liquid pipelines.
    pub fn is_liquid(self) -> bool {
        matches!(self, Self::Water | Self::Slime | Self::Lava)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureMetadata {
    pub frame_index: u8,
    pub alternate: bool,
    pub kind: Kind,
    pub group: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Indexed {
    pub name: String,
    pub metadata: TextureMetadata,
    pub dimensions: [u32; 2],
    /// Row-major palette indices for all four mip levels.
    pub mips: [Vec<u8>; MIP_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct External {
    pub name: String,
    pub metadata: TextureMetadata,
    pub dimensions: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Texture {
    Indexed(Indexed),
    External(External),
}

impl Texture {
    pub fn name(&self) -> &str {
        match self {
            Self::Indexed(texture) => &texture.name,
            Self::External(texture) => &texture.name,
        }
    }

    // Borrow the decoded metadata for either texture representation.
    fn metadata(&self) -> &TextureMetadata {
        match self {
            Self::Indexed(texture) => &texture.metadata,
            Self::External(texture) => &texture.metadata,
        }
    }

    pub fn kind(&self) -> Kind {
        self.metadata().kind
    }

    pub fn dimensions(&self) -> [u32; 2] {
        match self {
            Self::Indexed(texture) => texture.dimensions,
            Self::External(texture) => texture.dimensions,
        }
    }
}

// Decode each name into a texture kind and a position in its family.
fn decode_name(name: &str) -> TextureMetadata {
    let kind = if name
        .get(..SKY_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SKY_PREFIX))
    {
        Kind::Sky
    } else if name
        .get(..PORTAL_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PORTAL_PREFIX))
    {
        Kind::Portal
    } else if name
        .get(..SLIME_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SLIME_PREFIX))
    {
        Kind::Slime
    } else if name
        .get(..LAVA_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(LAVA_PREFIX))
    {
        Kind::Lava
    } else if name.starts_with(LIQUID_PREFIX) {
        Kind::Water
    } else {
        Kind::Solid
    };
    let (frame_index, alternate, group) = match name.as_bytes() {
        [FRAME_PREFIX, code @ b'0'..=b'9', _, ..] => (code - b'0', false, &name[2..]),
        [FRAME_PREFIX, code @ (b'a'..=b'j' | b'A'..=b'J'), _, ..] => {
            (code.to_ascii_lowercase() - b'a', true, &name[2..])
        }
        _ => (FIRST_FRAME, false, name),
    };
    TextureMetadata {
        frame_index,
        alternate,
        kind,
        group: group.to_owned(),
    }
}

// Group decoded names and map their member IDs to ordered frame sequences.
fn build_families(textures: &[Option<Texture>]) -> Families {
    let mut groups = HashMap::<_, Vec<_>>::new();
    for (id, texture) in textures.iter().enumerate() {
        let Some(texture) = texture else {
            continue;
        };
        let metadata = texture.metadata();
        // A plain texture and an animation can share the same base name.
        let animated = metadata.group != texture.name();
        groups
            .entry((animated, metadata.group.to_ascii_lowercase()))
            .or_default()
            .push((metadata, id));
    }

    let mut families = Families::new();
    for mut members in groups.into_values() {
        members
            .sort_unstable_by_key(|(metadata, id)| (metadata.alternate, metadata.frame_index, *id));
        let mut normal = Vec::new();
        let mut alternate = Vec::new();
        let mut previous = None;
        for (metadata, id) in &members {
            let frame = (metadata.alternate, metadata.frame_index);
            if previous == Some(frame) {
                continue;
            }
            previous = Some(frame);
            if metadata.alternate {
                alternate.push(*id);
            } else {
                normal.push(*id);
            }
        }
        if normal.is_empty() {
            normal.push(alternate[0]);
        }
        let family = Arc::new(Family {
            normal,
            alternate: (!alternate.is_empty()).then_some(alternate),
        });
        for (_, id) in members {
            families.insert(id, Arc::clone(&family));
        }
    }
    families
}

/// Decode BSP29 textures and derive their frame families.
/// Missing directory entries remain None so texture indices retain their original meaning.
pub fn decode(bytes: &[u8]) -> Result<(Vec<Option<Texture>>, Families)> {
    if bytes.is_empty() {
        return Ok((Vec::new(), Families::new()));
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
    let textures = offsets
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
            let metadata = decode_name(name);
            let dimensions = [
                u32::from_le_bytes(header[16..20].try_into().unwrap()),
                u32::from_le_bytes(header[20..24].try_into().unwrap()),
            ];
            if dimensions.contains(&0) {
                return Err(Error::new(ErrorKind::InvalidData, "zero texture dimension"));
            }
            if header[24..HEADER_SIZE].iter().all(|&byte| byte == 0) {
                return Ok(Some(Texture::External(External {
                    name: name.to_owned(),
                    metadata,
                    dimensions,
                })));
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
            Ok(Some(Texture::Indexed(Indexed {
                name: name.to_owned(),
                metadata,
                dimensions,
                mips,
            })))
        })
        .collect::<Result<Vec<_>>>()?;
    let families = build_families(&textures);
    Ok((textures, families))
}

#[cfg(test)]
mod test;
