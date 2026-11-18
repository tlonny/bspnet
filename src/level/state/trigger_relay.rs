use crate::bsp;

use super::delay::Delay;

#[derive(Debug, PartialEq, Eq)]
pub struct TriggerRelay {
    once: bool,
    spent: bool,
    delay: Delay,
}

impl TriggerRelay {
    /// Initialize a relay ready to forward an input.
    pub fn build(trigger: &bsp::TriggerRelay) -> Self {
        Self {
            once: trigger.once,
            spent: false,
            delay: Delay::build(trigger.delay),
        }
    }

    /// Forward one input and report whether its output should fire immediately.
    pub fn activate(&mut self) -> bool {
        if self.spent {
            return false;
        }
        self.spent = self.once;
        self.delay.activate()
    }

    /// Advance one simulation tick and return the number of delayed outputs due.
    pub fn update(&mut self) -> usize {
        self.delay.update()
    }
}
