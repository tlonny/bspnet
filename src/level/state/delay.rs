use std::collections::VecDeque;

use crate::level::STEP;

// Round a duration up to the first tick on which it has elapsed.
pub fn ticks(seconds: f32) -> u64 {
    (seconds / STEP.as_secs_f32()).ceil() as u64
}

#[derive(Debug, PartialEq, Eq)]
pub struct Delay {
    duration: u64,
    tick: u64,
    pending: VecDeque<u64>,
}

impl Delay {
    /// Initialize a queue for outputs that all use the same delay.
    pub fn build(seconds: f32) -> Self {
        Self {
            duration: ticks(seconds),
            tick: 0,
            pending: VecDeque::new(),
        }
    }

    /// Return true for an immediate output, or schedule it for a later tick.
    pub fn activate(&mut self) -> bool {
        if self.duration == 0 {
            return true;
        }
        self.pending
            .push_back(self.tick.saturating_add(self.duration));
        false
    }

    /// Advance a tick and return the number of outputs due.
    pub fn update(&mut self) -> usize {
        self.tick = self.tick.saturating_add(1);
        let mut due = 0;
        while self.pending.front().is_some_and(|&tick| tick <= self.tick) {
            self.pending.pop_front();
            due += 1;
        }
        due
    }
}
