use std::io::{Error, ErrorKind, Result};

const RECORD_SIZE: usize = 2;

/// Decode marksurfaces as face indices, preserving order and shared references.
pub fn decode(bytes: &[u8], face_count: usize) -> Result<Vec<usize>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "marksurfaces lump length must be a multiple of 2",
        ));
    }
    records
        .iter()
        .map(|record| {
            let face = usize::from(u16::from_le_bytes(*record));
            if face >= face_count {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "marksurface references missing face",
                ));
            }
            Ok(face)
        })
        .collect()
}

#[cfg(test)]
mod test;
