use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use super::property::{color as optional_color, float as optional_float};

const CLASSNAME: &str = "classname";
const WORLDSPAWN: &str = "worldspawn";
const WORLD_INDEX: usize = 0;
const SLIME_ALPHA: &str = "_slimealpha";
const SLIME_TINT: &str = "_slime_tint";
const SLIME_TINT_STRENGTH: &str = "_slime_tint_strength";
const WATER_ALPHA: &str = "_wateralpha";
const WATER_TINT: &str = "_water_tint";
const WATER_TINT_STRENGTH: &str = "_water_tint_strength";
const SKY_FOREGROUND_SPEED: &str = "_sky_foreground_speed";
const SKY_BACKGROUND_SPEED: &str = "_sky_background_speed";

#[derive(Debug, PartialEq)]
pub struct Worldspawn {
    pub slime_alpha: Option<f32>,
    pub slime_tint: Option<[f32; 3]>,
    pub slime_tint_strength: Option<f32>,
    pub water_alpha: Option<f32>,
    /// Underwater tint RGB, with components in 0..=1.
    pub water_tint: Option<[f32; 3]>,
    pub water_tint_strength: Option<f32>,
    /// Classic sky foreground scrolling speed in texture pixels per second.
    pub sky_foreground_speed: Option<f32>,
    /// Classic sky background scrolling speed in texture pixels per second.
    pub sky_background_speed: Option<f32>,
}

impl Worldspawn {
    /// Parse worldspawn at entity zero, ignoring unrecognised properties.
    pub fn parse(index: usize, properties: &HashMap<String, String>) -> Result<Self> {
        if index != WORLD_INDEX {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "worldspawn must be entity zero",
            ));
        }
        let classname = properties
            .get(CLASSNAME)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing worldspawn classname"))?;
        if classname != WORLDSPAWN {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "expected worldspawn classname",
            ));
        }
        let water_tint_strength = optional_float(properties, WATER_TINT_STRENGTH)?;
        if water_tint_strength.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid _water_tint_strength range",
            ));
        }
        let slime_tint_strength = optional_float(properties, SLIME_TINT_STRENGTH)?;
        if slime_tint_strength.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid _slime_tint_strength range",
            ));
        }
        Ok(Self {
            slime_alpha: optional_float(properties, SLIME_ALPHA)?,
            slime_tint: optional_color(properties, SLIME_TINT)?,
            slime_tint_strength,
            water_alpha: optional_float(properties, WATER_ALPHA)?,
            water_tint: optional_color(properties, WATER_TINT)?,
            water_tint_strength,
            sky_foreground_speed: optional_float(properties, SKY_FOREGROUND_SPEED)?,
            sky_background_speed: optional_float(properties, SKY_BACKGROUND_SPEED)?,
        })
    }
}

#[cfg(test)]
mod test;
