use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::ops::Range;

const BITS_PER_BYTE: usize = u8::BITS as usize;

/// Flattened visible leaf indices; solid leaf zero is excluded.
pub type Visibility = Vec<usize>;

/// Expand PVS rows and map their lump-relative compressed byte offsets to ranges.
pub fn decode(bytes: &[u8], visleafs: usize) -> Result<(Visibility, HashMap<usize, Range<usize>>)> {
    if !bytes.is_empty() && visleafs == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "nonempty visibility lump with no visibility leaves",
        ));
    }
    let row_bytes = visleafs.div_ceil(BITS_PER_BYTE);
    let mut visibility = Vec::new();
    let mut ranges = HashMap::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let offset = cursor;
        let start = visibility.len();
        let mut column = 0;
        while column < row_bytes {
            let value = *bytes
                .get(cursor)
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "truncated visibility row"))?;
            cursor += 1;
            if value == 0 {
                let count = usize::from(*bytes.get(cursor).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "truncated visibility zero run")
                })?);
                cursor += 1;
                if count == 0 || count > row_bytes - column {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "invalid visibility zero run length",
                    ));
                }
                column += count;
            } else {
                for bit in 0..BITS_PER_BYTE {
                    let index = column * BITS_PER_BYTE + bit;
                    if index < visleafs && value & (1 << bit) != 0 {
                        visibility.push(index + 1);
                    }
                }
                column += 1;
            }
        }
        ranges.insert(offset, start..visibility.len());
    }
    Ok((visibility, ranges))
}
