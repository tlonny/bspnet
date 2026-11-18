use std::time::Duration;

/// Fixed simulation interval: one hundred ticks per second.
pub const STEP: Duration = Duration::from_millis(10);

pub mod lightmap;
pub mod mesh;
pub mod properties;
pub mod state;
pub mod texture;
