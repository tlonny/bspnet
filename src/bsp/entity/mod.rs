use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till};
use nom::character::complete::{char, multispace1};
use nom::combinator::{all_consuming, value};
use nom::multi::{fold_many0, many0, many1};
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};

mod worldspawn;

pub use worldspawn::Worldspawn;

const CLASSNAME: &str = "classname";
const WORLDSPAWN: &str = "worldspawn";

#[derive(Debug, PartialEq)]
pub enum Entity {
    Worldspawn(Worldspawn),
    Unknown,
}

impl Entity {
    /// Parse recognised entities and discard properties of unsupported classes.
    pub fn parse(index: usize, properties: HashMap<String, String>) -> Result<Self> {
        let classname = properties
            .get(CLASSNAME)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing entity classname"))?;
        if index == 0 || classname == WORLDSPAWN {
            return Worldspawn::parse(index, &properties).map(Self::Worldspawn);
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
pub fn decode(bytes: &[u8]) -> Result<Vec<Entity>> {
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
        .map(|(index, properties)| Entity::parse(index, properties))
        .collect()
}
