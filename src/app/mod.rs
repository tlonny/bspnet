use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use glam::Vec2;

use bspnet::bsp;
use bspnet::gpu::{
    bind_group,
    buffer::{Index, uniform::Camera, vertex},
    pipeline,
    texture::Depth,
};
use bspnet::level::TextureRange;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

mod camera;
mod init;
mod render;
mod resize;
mod update;

const SIM_STEP: Duration = Duration::from_millis(10);

struct Graphics {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: Depth,
    pipeline: pipeline::World,
    camera: Camera,
    camera_binding: bind_group::Camera,
    vertices: vertex::World,
    indices: Index,
    textures: Vec<bind_group::Color>,
    ranges: Vec<TextureRange>,
}

pub struct App {
    last_update: Instant,
    window_title: String,
    graphics: Option<Graphics>,
    camera: camera::Camera,
    captured: bool,
    bsp: Option<bsp::Data>,
    leaf: Option<usize>,
}

impl App {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            window_title: String::new(),
            graphics: None,
            camera: camera::Camera::default(),
            captured: false,
            bsp: None,
            leaf: None,
        }
    }

    // Capture or release the cursor and clear pending camera input.
    fn capture_mouse(&mut self, captured: bool) {
        let Some(graphics) = &self.graphics else {
            return;
        };
        if captured {
            graphics
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| graphics.window.set_cursor_grab(CursorGrabMode::Confined))
                .expect("could not capture the mouse");
        } else {
            graphics
                .window
                .set_cursor_grab(CursorGrabMode::None)
                .unwrap();
        }
        graphics.window.set_cursor_visible(!captured);
        self.captured = captured;
        self.camera.clear_input();
    }

    /// Run the window event loop and fixed-step camera updates.
    pub fn run(mut self, title: &str) {
        self.window_title = title.to_owned();
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut self).unwrap();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_none() {
            self.init(event_loop);
        }
        self.last_update = Instant::now();
        self.graphics.as_ref().unwrap().window.request_redraw();
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            while self.last_update.elapsed() >= SIM_STEP {
                self.update();
                self.last_update += SIM_STEP;
            }
            if let Some(graphics) = &self.graphics {
                graphics.window.request_redraw();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::Resized(size) => self.resize(size.width, size.height),
            WindowEvent::Focused(false) => self.capture_mouse(false),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.capture_mouse(true),
            WindowEvent::Occluded(false) => {
                if let Some(graphics) = &self.graphics {
                    graphics.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if key == KeyCode::Escape && event.state.is_pressed() {
                        event_loop.exit();
                    } else if self.captured {
                        self.camera.key(key, event.state.is_pressed());
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if self.captured
            && let DeviceEvent::MouseMotion { delta } = event
        {
            self.camera
                .mouse_motion(Vec2::new(delta.0 as f32, delta.1 as f32));
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_update + SIM_STEP));
    }
}
