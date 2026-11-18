use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use super::property;

const MODEL: &str = "model";
const TARGET: &str = "target";
const TARGETNAME: &str = "targetname";
const WAIT: &str = "wait";
const DEFAULT_WAIT: f32 = 0.2;

#[derive(Debug, PartialEq)]
pub struct TriggerMultiple {
    /// Inline model defining the trigger volume.
    pub model: usize,
    pub target: Option<String>,
    pub targetname: Option<String>,
    /// Seconds between activation and firing the target.
    pub delay: f32,
    /// Seconds between accepted activations.
    pub wait: f32,
}

impl TriggerMultiple {
    /// Parse a repeatable trigger volume and its target wiring.
    pub fn parse(properties: &HashMap<String, String>, model_count: usize) -> Result<Self> {
        let model = property::model(properties, MODEL, model_count)?;
        let wait = property::float(properties, WAIT)?
            .filter(|&wait| wait != 0.0)
            .unwrap_or(DEFAULT_WAIT);
        if wait < 0.0 {
            return Err(Error::new(ErrorKind::InvalidData, "negative trigger wait"));
        }
        Ok(Self {
            model,
            target: property::string(properties, TARGET),
            targetname: property::string(properties, TARGETNAME),
            delay: property::delay(properties)?,
            wait,
        })
    }
}

#[cfg(test)]
mod test;
