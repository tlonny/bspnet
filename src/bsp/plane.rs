use std::io::{Error, ErrorKind, Result};

const RECORD_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: [f32; 3],
    pub distance: f32,
}

/// Decode BSP29 plane equations, ignoring the unused classification field.
pub fn decode(bytes: &[u8]) -> Result<Vec<Plane>> {
    let (records, remainder) = bytes.as_chunks::<RECORD_SIZE>();
    if !remainder.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "planes lump length must be a multiple of 20",
        ));
    }
    records
        .iter()
        .map(|record| {
            let normal = std::array::from_fn(|axis| {
                let start = axis * 4;
                f32::from_le_bytes(record[start..start + 4].try_into().unwrap())
            });
            let distance = f32::from_le_bytes(record[12..16].try_into().unwrap());
            if normal.iter().any(|value| !value.is_finite()) || !distance.is_finite() {
                return Err(Error::new(ErrorKind::InvalidData, "non-finite plane"));
            }
            Ok(Plane { normal, distance })
        })
        .collect()
}
