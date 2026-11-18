use std::collections::HashMap;
use std::io::Result;

use super::property;

const MODEL: &str = "model";
const TARGET: &str = "target";
const TARGETNAME: &str = "targetname";

#[derive(Debug, PartialEq)]
pub struct TriggerOnce {
    /// Inline model defining the trigger volume.
    pub model: usize,
    pub target: Option<String>,
    pub targetname: Option<String>,
    /// Seconds between activation and firing the target.
    pub delay: f32,
}

impl TriggerOnce {
    /// Parse a one-shot trigger volume and its target wiring.
    pub fn parse(properties: &HashMap<String, String>, model_count: usize) -> Result<Self> {
        let model = property::model(properties, MODEL, model_count)?;
        Ok(Self {
            model,
            target: property::string(properties, TARGET),
            targetname: property::string(properties, TARGETNAME),
            delay: property::delay(properties)?,
        })
    }
}

#[cfg(test)]
mod test;
