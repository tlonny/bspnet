use std::time::Duration;

use bspnet::{camera::Camera, level::STEP};
use glam::{Vec2, Vec3};
use winit::keyboard::KeyCode;

use super::super::App;

#[test]
fn fixed_updates_keep_partial_ticks_and_catch_up_without_rendering() {
    let mut app = App::new();
    app.camera = Camera::new(Vec3::ZERO, Vec2::ZERO);
    app.camera.key(KeyCode::KeyW, true);
    let start = app.last_update;
    app.advance(start + Duration::from_millis(9));
    assert_eq!(app.camera.position(), Vec3::ZERO);
    assert_eq!(app.last_update, start);
    app.advance(start + Duration::from_millis(25));
    assert_eq!(app.last_update, start + STEP * 2);
    assert!((app.camera.position().y - 6.4).abs() < 0.001);
    app.advance(start + Duration::from_millis(30));
    assert_eq!(app.last_update, start + STEP * 3);
    assert!((app.camera.position().y - 9.6).abs() < 0.001);
    let position = app.camera.position();
    app.advance(start + Duration::from_millis(30));
    assert_eq!(app.camera.position(), position);
    app.advance(start + Duration::from_millis(130));
    assert_eq!(app.last_update, start + STEP * 13);
    assert!((app.camera.position().y - 41.6).abs() < 0.001);
}

#[test]
fn one_tick_updates_the_camera_with_the_shared_interval() {
    let mut app = App::new();
    app.camera = Camera::new(Vec3::ZERO, Vec2::ZERO);
    app.camera.key(KeyCode::KeyD, true);
    app.update();
    assert!((app.camera.position().x - 3.2).abs() < 0.001);
    assert_eq!(app.camera.position().y, 0.);
}
