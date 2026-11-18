use super::Camera;
use glam::{Vec2, Vec3};
use winit::keyboard::KeyCode;

#[test]
fn diagonal_movement_has_the_same_speed_as_forward_movement() {
    let mut forward = Camera::default();
    forward.key(KeyCode::KeyW, true);
    forward.update();
    let mut diagonal = Camera::default();
    diagonal.key(KeyCode::KeyW, true);
    diagonal.key(KeyCode::KeyD, true);
    diagonal.update();
    assert!((forward.position().length() - diagonal.position().length()).abs() < 0.001);
    assert!(diagonal.position().x > 0. && diagonal.position().y > 0.);
}

#[test]
fn forward_movement_follows_yaw_and_pitch_in_z_up_coordinates() {
    let mut camera = Camera::new(
        Vec3::ZERO,
        Vec2::new(45f32.to_radians(), 90f32.to_radians()),
    );
    camera.key(KeyCode::KeyW, true);
    camera.update();
    assert!((camera.position().normalize() - Vec3::new(1., 0., 1.).normalize()).length() < 0.001);
}

#[test]
fn releasing_input_and_losing_focus_stops_movement_and_rotation() {
    let mut camera = Camera::default();
    camera.key(KeyCode::KeyW, true);
    camera.key(KeyCode::KeyW, false);
    camera.update();
    assert_eq!(camera.position(), Vec3::ZERO);
    camera.key(KeyCode::KeyD, true);
    camera.mouse_motion(Vec2::splat(100.));
    camera.clear_input();
    camera.update();
    assert_eq!(camera.position(), Vec3::ZERO);
    assert_eq!(camera.rotation(), Vec2::ZERO);
}

#[test]
fn mouse_motion_is_consumed_once_and_pitch_cannot_reach_vertical() {
    let mut camera = Camera::default();
    camera.mouse_motion(Vec2::new(100., -100_000.));
    camera.update();
    let rotation = camera.rotation();
    assert!(rotation.x > 0. && rotation.x < std::f32::consts::FRAC_PI_2);
    assert!(rotation.y > 0.);
    camera.update();
    assert_eq!(camera.rotation(), rotation);
}

#[test]
fn space_control_and_shift_allow_vertical_flight_at_two_speeds() {
    let mut normal = Camera::default();
    normal.key(KeyCode::Space, true);
    normal.update();
    assert!(normal.position().z > 0.);
    assert_eq!(normal.position().x, 0.);
    let mut fast = Camera::default();
    fast.key(KeyCode::ControlLeft, true);
    fast.key(KeyCode::ShiftLeft, true);
    fast.update();
    assert!(fast.position().z < -normal.position().z);
}
