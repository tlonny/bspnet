use std::io::{Error, ErrorKind, Result};

const MAGIC: &[u8; 4] = b"QLIT";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 8;
const SAMPLE_SIZE: usize = 3;

/// Validate a version-one .lit file and borrow its RGB lighting samples.
pub fn decode(bytes: &[u8]) -> Result<&[[u8; 3]]> {
    let header = bytes
        .get(..HEADER_SIZE)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated .lit header"))?;
    if &header[..4] != MAGIC {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "expected QLIT lighting file",
        ));
    }
    if u32::from_le_bytes(header[4..8].try_into().unwrap()) != VERSION {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "expected .lit version 1",
        ));
    }
    let (samples, remainder) = bytes[HEADER_SIZE..].as_chunks::<SAMPLE_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "partial RGB sample in .lit file",
        ));
    }
    Ok(samples)
}

#[cfg(test)]
mod test;
