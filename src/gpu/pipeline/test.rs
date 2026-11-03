use crate::gpu::{
    bind_group,
    buffer::{
        Index,
        uniform::{Camera, camera},
        vertex,
    },
    pipeline, render_pass,
    texture::{Color, Depth},
};
use crate::level::TextureRange;
use glam::{UVec2, Vec2, Vec3};

const WORLD_SHADER: &str = include_str!("../../../shader/world.wgsl");
const TARGET_SIZE: u32 = 4;
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const ROW_BYTES: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[test]
fn world_shader_validates() {
    let module = wgpu::naga::front::wgsl::parse_str(WORLD_SHADER).unwrap();
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn indexed_world_binds_textures_per_range_and_occludes_farther_triangles() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let pipeline = pipeline::World::create(&device, TARGET_FORMAT, WORLD_SHADER);
    let camera = Camera::create(&device);
    camera.write(&queue, camera::Data::construct(Vec3::ZERO, Vec2::ZERO, 1.0));
    let binding = bind_group::Camera::create(&device, &camera);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    let near = Color::create(&device, UVec2::new(2, 1));
    near.write(&queue, &[255, 0, 0, 255, 0, 255, 0, 255]);
    let far = Color::create(&device, UVec2::splat(1));
    far.write(&queue, &[0, 0, 255, 255]);
    let textures = [
        bind_group::Color::create(&device, &far, &sampler),
        bind_group::Color::create(&device, &near, &sampler),
        bind_group::Color::create(&device, &far, &sampler),
    ];
    let ranges = [
        TextureRange {
            model_id: 0,
            texture_id: 0,
            indices: 3..6,
        },
        TextureRange {
            model_id: 0,
            texture_id: 1,
            indices: 0..3,
        },
        TextureRange {
            model_id: 0,
            texture_id: 2,
            indices: 3..6,
        },
    ];
    let vertices = vertex::World::create(&device, 6);
    let mut indices = Index::create(&device, 6);
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("World test color"),
        size: wgpu::Extent3d {
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = color.create_view(&Default::default());
    let depth = Depth::create(&device, TARGET_SIZE, TARGET_SIZE);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("World test readback"),
        size: u64::from(ROW_BYTES * TARGET_SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = render_pass::World::create(&mut encoder, &view, depth.view());
        pass.draw(&pipeline, &binding, &vertices, &indices, &textures, &ranges);
    }
    vertices.write(
        &queue,
        &[
            vertex::world::Data {
                position: Vec3::new(-768.0, 512.0, -768.0),
                uv: Vec2::new(2.25, 0.5),
            },
            vertex::world::Data {
                position: Vec3::new(768.0, 512.0, -768.0),
                uv: Vec2::new(2.25, 0.5),
            },
            vertex::world::Data {
                position: Vec3::new(0.0, 512.0, 768.0),
                uv: Vec2::new(2.25, 0.5),
            },
            vertex::world::Data {
                position: Vec3::new(-1536.0, 1024.0, -1536.0),
                uv: Vec2::new(2.25, 0.5),
            },
            vertex::world::Data {
                position: Vec3::new(1536.0, 1024.0, -1536.0),
                uv: Vec2::new(2.25, 0.5),
            },
            vertex::world::Data {
                position: Vec3::new(0.0, 1024.0, 1536.0),
                uv: Vec2::new(2.25, 0.5),
            },
        ],
    );
    // Switch from far to near, then redraw far to check both texture binding and depth.
    indices.write(&queue, &[0, 1, 2, 3, 4, 5]);
    {
        let mut pass = render_pass::World::create(&mut encoder, &view, depth.view());
        pass.draw(&pipeline, &binding, &vertices, &indices, &textures, &ranges);
    }
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_BYTES),
                rows_per_image: Some(TARGET_SIZE),
            },
        },
        color.size(),
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback.map_async(wgpu::MapMode::Read, .., move |result| {
        sender.send(result).unwrap()
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let pixels = readback.get_mapped_range(..).unwrap();
    let center = (ROW_BYTES * (TARGET_SIZE / 2) + 4 * (TARGET_SIZE / 2)) as usize;
    assert!((191..=192).contains(&pixels[center]));
    assert_eq!(&pixels[center + 1..center + 3], &[0, 0]);
    assert_eq!(pixels[center + 3], 255);
}
