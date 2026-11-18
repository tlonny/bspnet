use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till};
use nom::character::complete::{char, multispace1};
use nom::combinator::{all_consuming, value};
use nom::multi::{fold_many0, many0, many1};
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};

mod button;
mod door;
mod portal;
mod property;
mod trigger_multiple;
mod trigger_once;
mod trigger_relay;
mod worldspawn;

pub use button::Button;
pub use door::Door;
pub use portal::{Geometry as PortalGeometry, Portal, Validation as PortalValidation};
pub use property::Angle;
pub use trigger_multiple::TriggerMultiple;
pub use trigger_once::TriggerOnce;
pub use trigger_relay::TriggerRelay;
pub use worldspawn::Worldspawn;

const CLASSNAME: &str = "classname";
const BUTTON: &str = "func_button";
const DOOR: &str = "func_door";
const PORTAL: &str = "func_portal";
const TRIGGER_ONCE: &str = "trigger_once";
const TRIGGER_MULTIPLE: &str = "trigger_multiple";
const TRIGGER_RELAY: &str = "trigger_relay";
const WALL: &str = "func_wall";
const ILLUSIONARY: &str = "func_illusionary";
const MODEL: &str = "model";
const OPAQUE: f32 = 1.0;
const WORLDSPAWN: &str = "worldspawn";

#[derive(Debug, PartialEq)]
pub struct StaticBrush {
    pub model: usize,
    pub alpha: Option<f32>,
}

#[derive(Debug, PartialEq)]
pub enum Entity {
    Worldspawn(Worldspawn),
    Door(Door),
    Portal(Portal),
    Button(Button),
    Wall(StaticBrush),
    Illusionary(StaticBrush),
    TriggerOnce(TriggerOnce),
    TriggerMultiple(TriggerMultiple),
    TriggerRelay(TriggerRelay),
    Unknown,
}

impl Entity {
    /// Return the inline model and effective opacity for a visible brush entity.
    pub fn brush(&self) -> Option<(usize, f32)> {
        let (model, alpha) = match self {
            Self::Door(door) => (door.model, door.alpha),
            Self::Portal(portal) => (portal.model?, None),
            Self::Button(button) => (button.model, button.alpha),
            Self::Wall(brush) | Self::Illusionary(brush) => (brush.model, brush.alpha),
            Self::Worldspawn(_)
            | Self::TriggerOnce(_)
            | Self::TriggerMultiple(_)
            | Self::TriggerRelay(_)
            | Self::Unknown => return None,
        };
        Some((model, alpha.unwrap_or(OPAQUE)))
    }

    /// Parse recognised entities and discard properties of unsupported classes.
    pub fn parse(
        index: usize,
        properties: HashMap<String, String>,
        bsp: &super::Data,
    ) -> Result<Self> {
        let classname = properties
            .get(CLASSNAME)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing entity classname"))?;
        if index == 0 || classname == WORLDSPAWN {
            return Worldspawn::parse(index, &properties).map(Self::Worldspawn);
        }
        if classname == PORTAL {
            return Ok(Self::Portal(Portal::parse(&properties, bsp)));
        }
        let model_count = bsp.models().len();
        if classname == DOOR {
            return Door::parse(index, &properties, model_count).map(Self::Door);
        }
        if classname == BUTTON {
            return Button::parse(index, &properties, model_count).map(Self::Button);
        }
        if classname == TRIGGER_ONCE {
            return TriggerOnce::parse(&properties, model_count).map(Self::TriggerOnce);
        }
        if classname == TRIGGER_MULTIPLE {
            return TriggerMultiple::parse(&properties, model_count).map(Self::TriggerMultiple);
        }
        if classname == TRIGGER_RELAY {
            return TriggerRelay::parse(&properties).map(Self::TriggerRelay);
        }
        if matches!(classname.as_str(), WALL | ILLUSIONARY) {
            let brush = StaticBrush {
                model: property::model(&properties, MODEL, model_count)?,
                alpha: property::alpha(&properties)?,
            };
            return Ok(if classname == WALL {
                Self::Wall(brush)
            } else {
                Self::Illusionary(brush)
            });
        }
        Ok(Self::Unknown)
    }
}

// Consume whitespace and Quake line comments between tokens.
fn ignored(text: &str) -> IResult<&str, ()> {
    value(
        (),
        many0(alt((
            multispace1,
            preceded(tag("//"), take_till(|character| character == '\n')),
        ))),
    )
    .parse(text)
}

// Read a quoted string, preserving literal backslashes as Quake does.
fn quoted(text: &str) -> IResult<&str, &str> {
    preceded(
        ignored,
        delimited(
            char('"'),
            take_till(|character| character == '"'),
            char('"'),
        ),
    )
    .parse(text)
}

// Collect one braced entity's properties, with the last duplicate key winning.
fn properties(text: &str) -> IResult<&str, HashMap<String, String>> {
    delimited(
        preceded(ignored, char('{')),
        fold_many0(
            (quoted, quoted),
            HashMap::new,
            |mut properties, (key, value)| {
                properties.insert(key.to_owned(), value.to_owned());
                properties
            },
        ),
        preceded(ignored, char('}')),
    )
    .parse(text)
}

/// Decode entity key/value pairs in lump order; later duplicate keys replace earlier values.
pub fn decode(bytes: &[u8], bsp: &super::Data) -> Result<Vec<Entity>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid entity text"))?
        .trim_end_matches('\0');
    if text.contains('\0') {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "embedded NUL in entity text",
        ));
    }
    let (_, entities) = all_consuming(terminated(many1(properties), ignored))
        .parse(text)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid entity lump syntax"))?;
    entities
        .into_iter()
        .enumerate()
        .map(|(index, properties)| Entity::parse(index, properties, bsp))
        .collect()
}

#[cfg(test)]
mod test;
