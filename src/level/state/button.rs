use crate::{bsp, level::STEP};

#[derive(Debug, PartialEq)]
enum Status {
    Released,
    Pressed { return_in: Option<f32> },
}

#[derive(Debug, PartialEq)]
pub struct Button {
    wait: Option<f32>,
    status: Status,
}

impl Button {
    /// Initialize the button in its normal texture state.
    pub fn build(button: &bsp::Button) -> Self {
        Self {
            wait: button.wait,
            status: Status::Released,
        }
    }

    /// Report whether the renderer should use the pressed texture.
    pub fn is_pressed(&self) -> bool {
        matches!(self.status, Status::Pressed { .. })
    }

    /// Enter the pressed state, keeping any countdown already in progress.
    pub fn press(&mut self) {
        if !self.is_pressed() {
            self.status = Status::Pressed {
                return_in: self.wait,
            };
        }
    }

    /// Restore the normal texture when the pressed timer expires.
    pub fn update(&mut self) {
        let Status::Pressed {
            return_in: Some(remaining),
        } = &mut self.status
        else {
            return;
        };
        let seconds = STEP.as_secs_f32();
        if *remaining <= seconds {
            self.status = Status::Released;
        } else {
            *remaining -= seconds;
        }
    }
}
