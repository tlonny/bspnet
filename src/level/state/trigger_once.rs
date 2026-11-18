use crate::bsp;

use super::delay;

#[derive(Debug, PartialEq, Eq)]
enum Status {
    Armed,
    Pending(u64),
    Spent,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TriggerOnce {
    delay_ticks: u64,
    status: Status,
}

impl TriggerOnce {
    /// Arm the trigger and convert its delay to whole simulation ticks.
    pub fn build(trigger: &bsp::TriggerOnce) -> Self {
        Self {
            delay_ticks: delay::ticks(trigger.delay),
            status: Status::Armed,
        }
    }

    /// Consume the first activation and return true if its target should fire immediately.
    pub fn activate(&mut self) -> bool {
        if self.status != Status::Armed {
            return false;
        }
        if self.delay_ticks == 0 {
            self.status = Status::Spent;
            return true;
        }
        self.status = Status::Pending(self.delay_ticks);
        false
    }

    /// Advance one simulation tick and return true once when a delayed target should fire.
    pub fn update(&mut self) -> bool {
        let Status::Pending(remaining) = &mut self.status else {
            return false;
        };
        *remaining -= 1;
        if *remaining == 0 {
            self.status = Status::Spent;
            return true;
        }
        false
    }
}
