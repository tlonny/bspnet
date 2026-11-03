use std::io::{Error, ErrorKind, Result};

const FACE_SIZE: usize = 20;

#[derive(Debug)]
pub struct Face {
    /// Starting index in the BSP's surfedges array.
    pub first_surfedge: i32,
    /// Number of consecutive surfedges forming the polygon boundary.
    pub num_surfedges: u16,
    /// Index into the BSP's texinfo array.
    pub texinfo: u16,
}

/// Read polygon ranges and texture mapping indices from BSP29 face records.
pub fn decode(bytes: &[u8]) -> Result<Vec<Face>> {
    if bytes.len() % FACE_SIZE != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "faces lump length must be a multiple of 20",
        ));
    }

    Ok(bytes
        .chunks_exact(FACE_SIZE)
        .map(|face| Face {
            first_surfedge: i32::from_le_bytes(face[4..8].try_into().unwrap()),
            num_surfedges: u16::from_le_bytes(face[8..10].try_into().unwrap()),
            texinfo: u16::from_le_bytes(face[10..12].try_into().unwrap()),
        })
        .collect())
}
