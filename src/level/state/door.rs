use crate::bsp::{self, Angle};
use crate::level::STEP;
use glam::Vec3;

use super::{INITIAL_TRANSLATION, destination};

#[derive(Debug, PartialEq, Eq)]
pub enum Motion {
    Closed,
    Opening,
    Open,
    Closing,
}

#[derive(Debug, PartialEq)]
pub struct Door {
    /// Current offset from the brush model's compiled position, in world units.
    pub translation: [f32; 3],
    /// Current movement direction, independent of the model's visual orientation.
    pub angle: Angle,
    pub speed: f32,
    pub lip: f32,
    pub motion: Motion,
    /// Seconds until automatic closing, when a return is scheduled.
    pub return_in: Option<f32>,
}

impl Door {
    /// Seed a closed door's runtime parameters from its parsed configuration.
    pub fn build(door: &bsp::Door) -> Self {
        Self {
            translation: INITIAL_TRANSLATION,
            angle: door.angle,
            speed: door.speed,
            lip: door.lip,
            motion: Motion::Closed,
            return_in: None,
        }
    }

    /// Open a closed door or reverse a closing door from its current position.
    pub fn activate(&mut self) {
        if matches!(self.motion, Motion::Closed | Motion::Closing) {
            self.motion = Motion::Opening;
            self.return_in = None;
        }
    }

    /// Advance movement and any scheduled return by exactly one simulation tick.
    pub fn update(&mut self, bsp: &bsp::Data, model_index: usize) {
        let seconds = STEP.as_secs_f32();
        let (target, finished) = match self.motion {
            Motion::Closed => return,
            Motion::Open => {
                if let Some(remaining) = self.return_in {
                    if remaining <= seconds {
                        self.return_in = None;
                        self.motion = Motion::Closing;
                    } else {
                        self.return_in = Some(remaining - seconds);
                    }
                }
                return;
            }
            Motion::Opening => {
                let Some(target) = destination(bsp, model_index, self.angle, self.lip) else {
                    return;
                };
                (target, Motion::Open)
            }
            Motion::Closing => (Vec3::ZERO, Motion::Closed),
        };
        let position = Vec3::from(self.translation);
        let delta = target - position;
        let distance = delta.length();
        let step = self.speed * seconds;
        if distance <= step {
            self.translation = target.to_array();
            self.motion = finished;
            if self.motion == Motion::Closed {
                self.return_in = None;
            }
        } else {
            self.translation = (position + delta * (step / distance)).to_array();
        }
    }
}
