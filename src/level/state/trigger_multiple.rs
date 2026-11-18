use crate::bsp;

use super::delay::{self, Delay};

#[derive(Debug, PartialEq, Eq)]
pub struct TriggerMultiple {
    wait_ticks: u64,
    cooldown: u64,
    delay: Delay,
}

impl TriggerMultiple {
    /// Initialize a trigger ready for its first activation.
    pub fn build(trigger: &bsp::TriggerMultiple) -> Self {
        Self {
            wait_ticks: delay::ticks(trigger.wait),
            cooldown: 0,
            delay: Delay::build(trigger.delay),
        }
    }

    /// Accept an activation outside the cooldown and report an immediate output.
    pub fn activate(&mut self) -> bool {
        if self.cooldown > 0 {
            return false;
        }
        self.cooldown = self.wait_ticks;
        self.delay.activate()
    }

    /// Advance the cooldown and return the number of delayed outputs due.
    pub fn update(&mut self) -> usize {
        self.cooldown = self.cooldown.saturating_sub(1);
        self.delay.update()
    }
}
