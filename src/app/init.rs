use std::{fs, sync::Arc};

use bspnet::{bsp, gpu, level};
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, window::Window};

use super::{App, Graphics};

const WINDOW_WIDTH: f64 = 1280.0;
const WINDOW_HEIGHT: f64 = 720.0;
const BSP_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/lq_e0m1.bsp"
);
const LIT_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/lq_e0m1.lit"
);
const PALETTE_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/palette.lmp"
);
const COMPOSITE_SHADER: &str = include_str!("../../shader/composite.wgsl");
const TINT_SHADER: &str = include_str!("../../shader/tint.wgsl");
const LIQUID_SHADER: &str = include_str!("../../shader/liquid.wgsl");
const SKY_SHADER: &str = include_str!("../../shader/sky.wgsl");
const SOLID_SHADER: &str = include_str!("../../shader/solid.wgsl");

impl App {
    /// Load the fixture, create GPU resources, and initialize world rendering.
    pub fn init(&mut self, event_loop: &ActiveEventLoop) {
        let bsp = bsp::Data::decode(&fs::read(BSP_FILE).expect("could not read LibreQuake BSP"))
            .expect("could not decode LibreQuake BSP");
        let lit = fs::read(LIT_FILE).expect("could not read LibreQuake .lit");
        let palette = fs::read(PALETTE_FILE).expect("could not read LibreQuake palette.lmp");
        let samples = bsp::sidecar::lit::decode(&lit).expect("could not decode LibreQuake .lit");
        let palette =
            bsp::sidecar::lmp::decode(&palette).expect("could not decode LibreQuake palette");
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
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface has no supported configuration");
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|format| {
                matches!(
                    format,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .expect("surface has no supported non-sRGB colour format");
        surface.configure(&device, &config);
        let depth = gpu::texture::Depth::create(&device, config.width, config.height);
        let camera_buffer = gpu::buffer::uniform::Camera::create(&device);
        let camera_binding = gpu::bind_group::Camera::create(&device, &camera_buffer);
        let pipeline = gpu::pipeline::solid::opaque::create(&device, config.format, SOLID_SHADER);
        let sky_pipeline = gpu::pipeline::sky::create(&device, config.format, SKY_SHADER);
        let liquid_pipeline =
            gpu::pipeline::liquid::opaque::create(&device, config.format, LIQUID_SHADER);
        let composite_pipeline =
            gpu::pipeline::composite::create(&device, config.format, COMPOSITE_SHADER);
        let transparent_liquid_pipeline =
            gpu::pipeline::liquid::transparent::create(&device, LIQUID_SHADER);
        let tint_pipeline = gpu::pipeline::tint::create(&device, config.format, TINT_SHADER);
        let accumulation = gpu::texture::Accumulation::create(&device, config.width, config.height);
        let revealage = gpu::texture::Revealage::create(&device, config.width, config.height);
        let composite_binding =
            gpu::bind_group::Composite::create(&device, &accumulation, &revealage);
        let tint_buffer = gpu::buffer::uniform::Tint::create(&device);
        let tint_binding = gpu::bind_group::Tint::create(&device, &tint_buffer);
        let properties_data = level::properties::build(&bsp);
        let properties_buffer = gpu::buffer::uniform::Properties::create(&device);
        properties_buffer.write(&queue, properties_data.water);
        let properties_binding = gpu::bind_group::Properties::create(&device, &properties_buffer);
        let slime_properties_buffer = gpu::buffer::uniform::Properties::create(&device);
        slime_properties_buffer.write(&queue, properties_data.slime);
        let slime_properties_binding =
            gpu::bind_group::Properties::create(&device, &slime_properties_buffer);
        let mut lightmap = level::lightmap::Builder::create(samples);
        let mut mesh = level::mesh::worldspawn::Mesh::build(&bsp, &mut lightmap, &device, &queue);
        let lightmaps = lightmap.upload(&device, &queue);
        let textures = bsp
            .textures()
            .iter()
            .enumerate()
            .map(|(index, source)| {
                source
                    .as_ref()
                    .map(|_| level::texture::build(&bsp, index, palette, &device, &queue))
            })
            .collect();
        let leaf = bsp
            .leaf_at(self.camera.position().to_array())
            .expect("invalid player-start leaf");
        mesh.set_leaf(&queue, leaf).unwrap();
        println!(
            "Loaded LibreQuake: {} faces, {} leaves; camera leaf {leaf}",
            bsp.faces().len(),
            bsp.leaves().len()
        );
        println!(
            "WASD to fly; Space/Ctrl up/down; Shift faster; click to toggle mouse look; Esc to quit."
        );
        self.graphics = Some(Graphics {
            window,
            instance,
            surface,
            device,
            queue,
            config,
            depth,
            camera_buffer,
            camera_binding,
            pipeline,
            sky_pipeline,
            liquid_pipeline,
            transparent_liquid_pipeline,
            composite_pipeline,
            tint_pipeline,
            accumulation,
            revealage,
            composite_binding,
            tint_buffer,
            tint_binding,
            properties_data,
            properties_buffer,
            properties_binding,
            slime_properties_buffer,
            slime_properties_binding,
            started: std::time::Instant::now(),
            bsp,
            mesh,
            lightmaps,
            textures,
        });
        self.resize(size.width, size.height);
    }
}
