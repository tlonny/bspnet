use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use super::property;

const TARGET: &str = "target";
const TARGETNAME: &str = "targetname";
const ONCE: &str = "once";
const REPEATABLE: &str = "0";
const ONE_SHOT: &str = "1";

#[derive(Debug, PartialEq)]
pub struct TriggerRelay {
    pub target: Option<String>,
    pub targetname: Option<String>,
    /// Seconds between activation and firing the target.
    pub delay: f32,
    /// Whether the first activation consumes the relay.
    pub once: bool,
}

impl TriggerRelay {
    /// Parse a point relay with an optional output delay and one-shot flag.
    pub fn parse(properties: &HashMap<String, String>) -> Result<Self> {
        let once = match properties.get(ONCE).map(|value| value.trim()) {
            None | Some(REPEATABLE) => false,
            Some(ONE_SHOT) => true,
            Some(_) => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "invalid relay once flag",
                ));
            }
        };
        Ok(Self {
            target: property::string(properties, TARGET),
            targetname: property::string(properties, TARGETNAME),
            delay: property::delay(properties)?,
            once,
        })
    }
}

#[cfg(test)]
mod test;
