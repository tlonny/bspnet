use glam::{Vec2, Vec3};
use winit::keyboard::KeyCode;

use super::{Camera, PITCH_LIMIT, SPEED};

#[test]
fn diagonal_movement_has_the_same_speed_as_forward_movement() {
    let mut camera = Camera::default();
    camera.key(KeyCode::KeyW, true);
    camera.update(1.0);
    assert_eq!(camera.position(), Vec3::Y * SPEED);
    let mut camera = Camera::default();
    camera.key(KeyCode::KeyW, true);
    camera.key(KeyCode::KeyD, true);
    camera.update(1.0);
    assert!((camera.position().length() - SPEED).abs() < 0.001);
    assert!(camera.position().x > 0.0 && camera.position().y > 0.0);
}

#[test]
fn forward_movement_follows_yaw_and_pitch_without_collisions() {
    let rotation = Vec2::new(45_f32.to_radians(), 90_f32.to_radians());
    let mut camera = Camera::new(Vec3::ZERO, rotation);
    camera.key(KeyCode::KeyW, true);
    camera.update(1.0);
    let expected = Vec3::new(1.0, 0.0, 1.0).normalize() * SPEED;
    assert!((camera.position() - expected).length() < 0.001);
}

#[test]
fn released_or_cleared_input_stops_movement_and_mouse_rotation() {
    let mut camera = Camera::default();
    camera.key(KeyCode::KeyW, true);
    camera.key(KeyCode::KeyW, false);
    camera.update(1.0);
    assert_eq!(camera.position(), Vec3::ZERO);
    camera.key(KeyCode::KeyD, true);
    camera.mouse_motion(Vec2::splat(100.0));
    camera.clear_input();
    camera.update(1.0);
    assert_eq!(camera.position(), Vec3::ZERO);
    assert_eq!(camera.rotation(), Vec2::ZERO);
}

#[test]
fn mouse_motion_is_consumed_once_and_pitch_stays_below_vertical() {
    let mut camera = Camera::default();
    camera.mouse_motion(Vec2::new(100.0, -100_000.0));
    camera.update(0.0);
    assert_eq!(camera.rotation().x, PITCH_LIMIT);
    assert!(camera.rotation().y > 0.0);
    let rotation = camera.rotation();
    camera.update(0.0);
    assert_eq!(camera.rotation(), rotation);
}
