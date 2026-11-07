use std::io::{Error, ErrorKind, Result};

const COLOR_COUNT: usize = 256;
const COLOR_SIZE: usize = 3;
const PALETTE_SIZE: usize = COLOR_COUNT * COLOR_SIZE;

/// Decode palette.lmp as exactly 256 RGB colors, with no file header.
pub fn decode(bytes: &[u8]) -> Result<&[[u8; 3]; COLOR_COUNT]> {
    if bytes.len() != PALETTE_SIZE {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "expected 768-byte palette.lmp",
        ));
    }
    let (colors, _) = bytes.as_chunks::<COLOR_SIZE>();
    Ok(colors.try_into().unwrap())
}
