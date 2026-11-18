use std::time::Instant;

use bspnet::{
    bsp::Entity,
    level::{STEP, state::State},
};

use super::App;

impl App {
    /// Advance the camera and each supported entity by one fixed simulation tick.
    pub fn update(&mut self) {
        self.camera.update();
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        for (entity, state) in graphics.bsp.entities().iter().zip(&mut graphics.states) {
            match (entity, state) {
                (Entity::Door(config), State::Door(door)) => {
                    door.update(&graphics.bsp, config.model);
                }
                (Entity::Button(_), State::Button(button)) => {
                    button.update();
                }
                _ => {}
            }
        }
    }

    /// Run every elapsed tick and retain any partial interval for the next event batch.
    pub fn advance(&mut self, now: Instant) {
        while now.saturating_duration_since(self.last_update) >= STEP {
            self.update();
            self.last_update += STEP;
        }
    }
}

#[cfg(test)]
mod test;
