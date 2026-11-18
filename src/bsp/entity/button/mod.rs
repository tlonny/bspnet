use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use super::property;

const CLASSNAME: &str = "classname";
const BUTTON: &str = "func_button";
const WORLD_INDEX: usize = 0;
const MODEL: &str = "model";
const TARGET: &str = "target";
const WAIT: &str = "wait";
const DEFAULT_WAIT: f32 = 1.0;

#[derive(Debug, PartialEq)]
pub struct Button {
    pub model: usize,
    pub alpha: Option<f32>,
    pub target: Option<String>,
    /// Seconds to remain pressed; None means it does not return automatically.
    pub wait: Option<f32>,
}

impl Button {
    /// Decode supported button properties, applying Quake defaults and ignoring extra keys.
    pub fn parse(
        index: usize,
        properties: &HashMap<String, String>,
        model_count: usize,
    ) -> Result<Self> {
        if index == WORLD_INDEX {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "func_button cannot be entity zero",
            ));
        }
        if properties.get(CLASSNAME).map(String::as_str) != Some(BUTTON) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "expected func_button classname",
            ));
        }
        let wait = property::float(properties, WAIT)?
            .filter(|&value| value != 0.0)
            .unwrap_or(DEFAULT_WAIT);
        Ok(Self {
            model: property::model(properties, MODEL, model_count)?,
            alpha: property::alpha(properties)?,
            target: property::string(properties, TARGET),
            wait: (wait >= 0.0).then_some(wait),
        })
    }
}

#[cfg(test)]
mod test;
