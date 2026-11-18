use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

const UP: f32 = -1.0;
const DOWN: f32 = -2.0;
const ALPHA: &str = "alpha";
const DELAY: &str = "delay";
const DEFAULT_DELAY: f32 = 0.0;
const MODEL_PREFIX: char = '*';

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Angle {
    Up,
    Down,
    /// Horizontal movement direction in degrees; zero points along +X.
    Yaw(f32),
}

// Parse an optional finite float while leaving absent overrides unset.
pub fn float(properties: &HashMap<String, String>, key: &str) -> Result<Option<f32>> {
    properties
        .get(key)
        .map(|value| {
            value
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, format!("invalid {key} float")))
        })
        .transpose()
}

// Parse a trigger's output delay in seconds.
pub fn delay(properties: &HashMap<String, String>) -> Result<f32> {
    let delay = float(properties, DELAY)?.unwrap_or(DEFAULT_DELAY);
    if delay < 0.0 {
        return Err(Error::new(ErrorKind::InvalidData, "negative trigger delay"));
    }
    Ok(delay)
}

// Parse an optional opacity in the normalised alpha range.
pub fn alpha(properties: &HashMap<String, String>) -> Result<Option<f32>> {
    let alpha = float(properties, ALPHA)?;
    if alpha.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
        return Err(Error::new(ErrorKind::InvalidData, "alpha outside 0..=1"));
    }
    Ok(alpha)
}

// Parse exactly three finite RGB components in the normalised colour range.
pub fn color(properties: &HashMap<String, String>, key: &str) -> Result<Option<[f32; 3]>> {
    properties
        .get(key)
        .map(|value| {
            let mut components = value.split_whitespace();
            let mut color = [0.; 3];
            for component in &mut color {
                *component = components
                    .next()
                    .and_then(|value| value.parse::<f32>().ok())
                    .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                    .ok_or_else(|| {
                        Error::new(ErrorKind::InvalidData, format!("invalid {key} color"))
                    })?;
            }
            if components.next().is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid {key} color"),
                ));
            }
            Ok(color)
        })
        .transpose()
}

// Read a nonempty optional string without interpreting its contents.
pub fn string(properties: &HashMap<String, String>, key: &str) -> Option<String> {
    properties
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned()
}

// Decode Quake's vertical angle sentinels before exposing the property.
pub fn angle(properties: &HashMap<String, String>, key: &str) -> Result<Option<Angle>> {
    Ok(float(properties, key)?.map(|value| match value {
        UP => Angle::Up,
        DOWN => Angle::Down,
        yaw => Angle::Yaw(yaw),
    }))
}

// Require an inline brush-model reference and validate it against the model lump.
pub fn model(properties: &HashMap<String, String>, key: &str, model_count: usize) -> Result<usize> {
    let value = properties
        .get(key)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, format!("missing {key} property")))?;
    let index = value
        .trim()
        .strip_prefix(MODEL_PREFIX)
        .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|digits| digits.parse::<usize>().ok())
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, format!("invalid {key} brush model")))?;
    if index == 0 || index >= model_count {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("{key} references missing brush model"),
        ));
    }
    Ok(index)
}
