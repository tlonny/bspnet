use std::io::{Error, ErrorKind, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contents {
    Empty,
    Solid,
    Water,
    Slime,
    Lava,
    Sky,
    Clip,
}

impl Contents {
    /// Decode a supported BSP contents code.
    pub fn decode(value: i32) -> Result<Self> {
        match value {
            -1 => Ok(Self::Empty),
            -2 => Ok(Self::Solid),
            -3 => Ok(Self::Water),
            -4 => Ok(Self::Slime),
            -5 => Ok(Self::Lava),
            -6 => Ok(Self::Sky),
            -8 => Ok(Self::Clip),
            _ => Err(Error::new(ErrorKind::InvalidData, "invalid contents")),
        }
    }
}
