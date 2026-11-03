use std::sync::Arc;

use bspnet::{
    bsp,
    gpu::{
        bind_group,
        buffer::{Index, uniform::Camera, vertex},
        pipeline,
        texture::{Color, Depth},
    },
    level,
};
use glam::{Vec2, Vec3};
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, window::Window};

use super::{App, Graphics, camera};

const WINDOW_WIDTH: f64 = 1280.0;
const WINDOW_HEIGHT: f64 = 720.0;
const WORLD_SHADER: &str = include_str!("../../shader/world.wgsl");
const BSP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/lq_e0m1-notex.bsp"
);
const TEXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/textures/librequake");
// The bundled map's player spawn plus Quake's 22-unit eye height.
const CAMERA_POSITION: Vec3 = Vec3::new(512.0, 2048.0, 326.0);
// Quake yaw 15 degrees becomes 75 degrees for our +Y-forward camera.
const CAMERA_ROTATION: Vec2 = Vec2::new(0.0, 75_f32.to_radians());

impl App {
    /// Load and triangulate the BSP, then create the window and upload its mesh.
    pub fn init(&mut self, event_loop: &ActiveEventLoop) {
        let bytes = std::fs::read(BSP_PATH).expect("could not read bundled BSP");
        let bsp = bsp::Data::decode(&bytes).expect("could not decode bundled BSP");
        let mesh = level::triangulate(&bsp).expect("could not triangulate BSP");
        self.camera = camera::Camera::new(CAMERA_POSITION, CAMERA_ROTATION);
        let attributes = Window::default_attributes()
            .with_title(&self.window_title)
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("no compatible graphics adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
            .expect("could not create graphics device");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let textures = bsp
            .textures
            .iter()
            .map(|texture| {
                let path = std::path::Path::new(TEXTURE_PATH).join(format!("{}.png", texture.name));
                let pixels = image::open(&path)
                    .unwrap_or_else(|error| {
                        panic!("could not load texture {}: {error}", path.display())
                    })
                    .into_rgba8();
                assert_eq!(
                    glam::UVec2::new(pixels.width(), pixels.height()),
                    texture.dimensions,
                    "texture dimensions disagree with BSP for {}",
                    texture.name
                );
                let color = Color::create(&device, texture.dimensions);
                color.write(&queue, pixels.as_raw());
                bind_group::Color::create(&device, &color, &sampler)
            })
            .collect();
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface has no supported configuration");
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(config.format);
        surface.configure(&device, &config);
        let depth = Depth::create(&device, config.width, config.height);
        let pipeline = pipeline::World::create(&device, config.format, WORLD_SHADER);
        let camera = Camera::create(&device);
        let camera_binding = bind_group::Camera::create(&device, &camera);
        let vertices = vertex::World::create(&device, mesh.vertices.len());
        vertices.write(&queue, &mesh.vertices);
        let mut indices = Index::create(&device, mesh.indices.len());
        indices.write(&queue, &mesh.indices);
        println!(
            "Loaded lq_e0m1-notex.bsp: {} vertices, {} triangles",
            mesh.vertices.len(),
            mesh.indices.len() / 3
        );
        println!("Click to capture the mouse. WASD to fly, mouse to look, Escape to quit.");
        self.graphics = Some(Graphics {
            window,
            instance,
            surface,
            device,
            queue,
            config,
            depth,
            pipeline,
            camera,
            camera_binding,
            vertices,
            indices,
            textures,
            ranges: mesh.ranges,
        });
        self.bsp = Some(bsp);
        self.leaf = None;
        self.resize(size.width, size.height);
    }
}
