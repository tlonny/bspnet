use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

const VERSION: i32 = 29;
const INTEGER_SIZE: usize = 4;
const ENTRY_SIZE: usize = 8;
const LUMP_COUNT: usize = 15;
const HEADER_SIZE: usize = INTEGER_SIZE + ENTRY_SIZE * LUMP_COUNT;
#[derive(Debug)]
pub struct Data {
    pub entities: Range<usize>,
    pub planes: Range<usize>,
    pub textures: Range<usize>,
    pub marksurfaces: Range<usize>,
    pub vertices: Range<usize>,
    pub visibility: Range<usize>,
    pub nodes: Range<usize>,
    pub texinfo: Range<usize>,
    pub edges: Range<usize>,
    pub surfedges: Range<usize>,
    pub faces: Range<usize>,
    pub models: Range<usize>,
    pub clipnodes: Range<usize>,
    pub leaves: Range<usize>,
}

/// Decode the BSP29 directory into named file ranges.
pub fn decode(bytes: &[u8]) -> Result<Data> {
    let header = bytes
        .get(..HEADER_SIZE)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated BSP header"))?;
    let version = i32::from_le_bytes(header[..INTEGER_SIZE].try_into().unwrap());
    if version != VERSION {
        return Err(Error::new(ErrorKind::InvalidData, "expected Quake BSP29"));
    }
    let mut ranges: [Range<usize>; LUMP_COUNT] = std::array::from_fn(|_| 0..0);
    for (range, entry) in ranges
        .iter_mut()
        .zip(header[INTEGER_SIZE..].chunks_exact(ENTRY_SIZE))
    {
        let offset = i32::from_le_bytes(entry[..INTEGER_SIZE].try_into().unwrap());
        let length = i32::from_le_bytes(entry[INTEGER_SIZE..].try_into().unwrap());
        if offset < HEADER_SIZE as i32 || length < 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid lump offset or length",
            ));
        }
        let start = offset as usize;
        let end = start
            .checked_add(length as usize)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "lump range too large"))?;
        if end > bytes.len() {
            return Err(Error::new(ErrorKind::InvalidData, "lump outside BSP file"));
        }
        *range = start..end;
    }
    let [
        entities,
        planes,
        textures,
        vertices,
        visibility,
        nodes,
        texinfo,
        faces,
        _,
        clipnodes,
        leaves,
        marksurfaces,
        edges,
        surfedges,
        models,
    ] = ranges;
    Ok(Data {
        entities,
        planes,
        textures,
        marksurfaces,
        vertices,
        visibility,
        nodes,
        texinfo,
        edges,
        surfedges,
        faces,
        models,
        clipnodes,
        leaves,
    })
}

#[cfg(test)]
mod test;
