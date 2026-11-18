use std::{collections::HashSet, f32::consts::TAU};

use crate::level::STEP;
use glam::{Vec2, Vec3};
use winit::keyboard::KeyCode;

const FAST_MULTIPLIER: f32 = 4.0;
const SPEED: f32 = 320.0;
const MOUSE_SENSITIVITY: f32 = 0.002;
const PITCH_LIMIT: f32 = 1.53589;

#[derive(Default)]
pub struct Camera {
    position: Vec3,
    rotation: Vec2,
    keys: HashSet<KeyCode>,
    mouse_delta: Vec2,
}

impl Camera {
    pub fn new(position: Vec3, rotation: Vec2) -> Self {
        Self {
            position,
            rotation,
            ..Self::default()
        }
    }

    // Compute the viewing direction in Quake's Z-up coordinate system.
    fn forward(&self) -> Vec3 {
        let (pitch_sin, pitch_cos) = self.rotation.x.sin_cos();
        let (yaw_sin, yaw_cos) = self.rotation.y.sin_cos();
        Vec3::new(yaw_sin * pitch_cos, yaw_cos * pitch_cos, pitch_sin)
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn rotation(&self) -> Vec2 {
        self.rotation
    }

    /// Track held movement keys.
    pub fn key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    /// Accumulate relative mouse movement until the next simulation tick.
    pub fn mouse_motion(&mut self, delta: Vec2) {
        self.mouse_delta += delta;
    }

    /// Release input when the cursor is released or the window loses focus.
    pub fn clear_input(&mut self) {
        self.keys.clear();
        self.mouse_delta = Vec2::ZERO;
    }

    /// Fly along the view direction with normalized WASD input and no collisions.
    pub fn update(&mut self) {
        self.rotation.x = (self.rotation.x - self.mouse_delta.y * MOUSE_SENSITIVITY)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.rotation.y =
            (self.rotation.y + self.mouse_delta.x * MOUSE_SENSITIVITY).rem_euclid(TAU);
        self.mouse_delta = Vec2::ZERO;
        let (yaw_sin, yaw_cos) = self.rotation.y.sin_cos();
        let right = Vec3::new(yaw_cos, -yaw_sin, 0.0);
        let mut direction = Vec3::ZERO;
        if self.keys.contains(&KeyCode::KeyW) {
            direction += self.forward();
        }
        if self.keys.contains(&KeyCode::KeyS) {
            direction -= self.forward();
        }
        if self.keys.contains(&KeyCode::KeyD) {
            direction += right;
        }
        if self.keys.contains(&KeyCode::KeyA) {
            direction -= right;
        }
        if self.keys.contains(&KeyCode::Space) {
            direction += Vec3::Z;
        }
        if self.keys.contains(&KeyCode::ControlLeft) {
            direction -= Vec3::Z;
        }
        let speed = if self.keys.contains(&KeyCode::ShiftLeft) {
            SPEED * FAST_MULTIPLIER
        } else {
            SPEED
        };
        self.position += direction.normalize_or_zero() * speed * STEP.as_secs_f32();
    }
}

#[cfg(test)]
mod test;
