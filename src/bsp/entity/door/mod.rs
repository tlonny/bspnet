use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use super::property::{self, Angle};

const CLASSNAME: &str = "classname";
const DOOR: &str = "func_door";
const WORLD_INDEX: usize = 0;
const MODEL: &str = "model";
const ANGLE: &str = "angle";
const SPEED: &str = "speed";
const LIP: &str = "lip";
const TARGETNAME: &str = "targetname";
const DEFAULT_SPEED: f32 = 100.0;
const DEFAULT_LIP: f32 = 8.0;
const DEFAULT_ANGLE: Angle = Angle::Yaw(0.0);

#[derive(Debug, PartialEq)]
pub struct Door {
    pub model: usize,
    pub alpha: Option<f32>,
    /// Movement direction, rather than the brush model's visual rotation.
    pub angle: Angle,
    /// Movement speed in world units per second.
    pub speed: f32,
    /// Distance left protruding at the end of travel, in world units.
    pub lip: f32,
    pub targetname: Option<String>,
}

impl Door {
    /// Decode a sliding door, ignoring unsupported properties and applying Quake defaults.
    pub fn parse(
        index: usize,
        properties: &HashMap<String, String>,
        model_count: usize,
    ) -> Result<Self> {
        if index == WORLD_INDEX {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "func_door cannot be entity zero",
            ));
        }
        if properties.get(CLASSNAME).map(String::as_str) != Some(DOOR) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "expected func_door classname",
            ));
        }
        // Quake treats explicit zero as unset for both speed and lip.
        let speed = property::float(properties, SPEED)?
            .filter(|&value| value != 0.0)
            .unwrap_or(DEFAULT_SPEED);
        if speed < 0.0 {
            return Err(Error::new(ErrorKind::InvalidData, "negative door speed"));
        }
        Ok(Self {
            model: property::model(properties, MODEL, model_count)?,
            alpha: property::alpha(properties)?,
            angle: property::angle(properties, ANGLE)?.unwrap_or(DEFAULT_ANGLE),
            speed,
            lip: property::float(properties, LIP)?
                .filter(|&value| value != 0.0)
                .unwrap_or(DEFAULT_LIP),
            targetname: property::string(properties, TARGETNAME),
        })
    }
}

#[cfg(test)]
mod test;
