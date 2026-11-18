use crate::bsp::{self, Angle, Entity};
use glam::Vec3;

mod button;
mod delay;
mod door;
mod trigger_multiple;
mod trigger_once;
mod trigger_relay;

pub use button::Button;
pub use door::{Door, Motion};
pub use trigger_multiple::TriggerMultiple;
pub use trigger_once::TriggerOnce;
pub use trigger_relay::TriggerRelay;

const INITIAL_TRANSLATION: [f32; 3] = [0.; 3];

// Find the travel displacement from the model's bounds and the current movement parameters.
fn destination(bsp: &bsp::Data, model_index: usize, angle: Angle, lip: f32) -> Option<Vec3> {
    let model = bsp.models().get(model_index)?;
    let [min, max] = model.bounds.map(Vec3::from);
    let direction = match angle {
        Angle::Up => Vec3::Z,
        Angle::Down => Vec3::NEG_Z,
        Angle::Yaw(degrees) => {
            let (sin, cos) = degrees.to_radians().sin_cos();
            Vec3::new(cos, sin, 0.)
        }
    };
    Some(direction * (direction.abs().dot(max - min) - lip))
}

#[derive(Debug, PartialEq)]
pub enum State {
    Empty,
    Door(Door),
    Button(Button),
    TriggerOnce(TriggerOnce),
    TriggerMultiple(TriggerMultiple),
    TriggerRelay(TriggerRelay),
}

impl State {
    /// Initialize per-entity runtime state while preserving the entity's BSP configuration.
    pub fn build(entity: &Entity) -> Self {
        match entity {
            Entity::Door(door) => Self::Door(Door::build(door)),
            Entity::Button(button) => Self::Button(Button::build(button)),
            Entity::TriggerOnce(trigger) => Self::TriggerOnce(TriggerOnce::build(trigger)),
            Entity::TriggerMultiple(trigger) => {
                Self::TriggerMultiple(TriggerMultiple::build(trigger))
            }
            Entity::TriggerRelay(trigger) => Self::TriggerRelay(TriggerRelay::build(trigger)),
            Entity::Worldspawn(_)
            | Entity::Unknown
            | Entity::Wall(_)
            | Entity::Illusionary(_)
            | Entity::Portal(_) => Self::Empty,
        }
    }
}

#[cfg(test)]
mod test;
