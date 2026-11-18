use std::{sync::Arc, time::Instant};

use bspnet::{bsp, camera, gpu, level};
use glam::{Vec2, Vec3};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, MouseButton, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

mod init;
mod render;
mod resize;
mod update;

const PLAYER_START: Vec3 = Vec3::new(512.0, 2048.0, 326.0);
const PLAYER_ROTATION: Vec2 = Vec2::new(0.0, 75f32.to_radians());

struct Graphics {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: gpu::texture::Depth,
    camera_buffer: gpu::buffer::uniform::Camera,
    camera_binding: gpu::bind_group::Camera,
    pipeline: wgpu::RenderPipeline,
    transparent_solid_pipeline: wgpu::RenderPipeline,
    entity_opacity: Vec<Option<wgpu::BindGroup>>,
    sky_pipeline: wgpu::RenderPipeline,
    liquid_pipeline: wgpu::RenderPipeline,
    transparent_liquid_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    tint_pipeline: wgpu::RenderPipeline,
    accumulation: gpu::texture::Accumulation,
    revealage: gpu::texture::Revealage,
    composite_binding: gpu::bind_group::Composite,
    tint_buffer: gpu::buffer::uniform::Tint,
    tint_binding: gpu::bind_group::Tint,
    properties_data: level::properties::Data,
    properties_buffer: gpu::buffer::uniform::Properties,
    properties_binding: gpu::bind_group::Properties,
    slime_properties_buffer: gpu::buffer::uniform::Properties,
    slime_properties_binding: gpu::bind_group::Properties,
    started: Instant,
    bsp: bsp::Data,
    states: Vec<level::state::State>,
    mesh: level::mesh::worldspawn::Mesh,
    entity_meshes: Vec<Option<level::mesh::entity::Mesh>>,
    lightmaps: gpu::bind_group::Array,
    textures: Vec<Option<level::texture::Texture>>,
}

pub struct App {
    window_title: String,
    graphics: Option<Graphics>,
    camera: camera::Camera,
    cursor_captured: bool,
    last_update: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            window_title: String::new(),
            graphics: None,
            camera: camera::Camera::new(PLAYER_START, PLAYER_ROTATION),
            cursor_captured: false,
            last_update: Instant::now(),
        }
    }

    // Capture relative mouse input while the user is looking around the level.
    fn capture_cursor(&mut self, capture: bool) {
        let Some(graphics) = &self.graphics else {
            return;
        };
        let mode = if capture {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        if let Err(error) = graphics.window.set_cursor_grab(mode) {
            eprintln!("Could not change cursor capture: {error}");
            return;
        }
        graphics.window.set_cursor_visible(!capture);
        self.cursor_captured = capture;
        self.camera.clear_input();
    }

    /// Run the LibreQuake viewer's window event loop.
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

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.advance(Instant::now());
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_update + level::STEP));
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if self.cursor_captured {
            if let DeviceEvent::MouseMotion { delta: (x, y) } = event {
                self.camera.mouse_motion(Vec2::new(x as f32, y as f32));
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::Resized(size) => self.resize(size.width, size.height),
            WindowEvent::Focused(false) => {
                self.capture_cursor(false);
                self.camera.clear_input();
            }
            WindowEvent::Occluded(false) => {
                if let Some(graphics) = &self.graphics {
                    graphics.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } if state.is_pressed() => {
                self.capture_cursor(!self.cursor_captured);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if key == KeyCode::Escape && event.state.is_pressed() {
                        event_loop.exit();
                    } else {
                        self.camera.key(key, event.state.is_pressed());
                    }
                }
            }
            _ => {}
        }
    }
}
