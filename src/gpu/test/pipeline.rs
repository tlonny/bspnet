use crate::bsp::texture::Kind;
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
use glam::{Vec2, Vec3};

const SOLID_SHADER: &str = include_str!("../../../shader/solid.wgsl");
const TARGET_SIZE: u32 = 4;
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const ROW_BYTES: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[test]
fn world_shader_validates() {
    for shader in [
        SOLID_SHADER,
        include_str!("../../../shader/sky.wgsl"),
        include_str!("../../../shader/liquid.wgsl"),
        include_str!("../../../shader/composite.wgsl"),
        include_str!("../../../shader/tint.wgsl"),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(shader).unwrap();
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}

// Draw overlapping textured triangles with selected lightmap layers and read their center pixel.
fn assert_world_lighting(
    lightmap_layers: [u32; 4],
    atlases: &[Vec<[u8; 4]>],
    expected_red: Option<u8>,
    kind: Kind,
    seconds: f32,
) -> u8 {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let pipeline = pipeline::solid::opaque::create(&device, TARGET_FORMAT, SOLID_SHADER);
    let sky_pipeline = pipeline::sky::create(
        &device,
        TARGET_FORMAT,
        include_str!("../../../shader/sky.wgsl"),
    );
    let liquid_pipeline = pipeline::liquid::opaque::create(
        &device,
        TARGET_FORMAT,
        include_str!("../../../shader/liquid.wgsl"),
    );
    let properties_buffer = crate::gpu::buffer::uniform::Properties::create(&device);
    let mut data = crate::gpu::buffer::uniform::properties::Data::construct(0., 0., 1., [0.; 4]);
    data.seconds = seconds;
    properties_buffer.write(&queue, data);
    let properties = bind_group::Properties::create(&device, &properties_buffer);
    let selected_pipeline = match kind {
        Kind::Solid => &pipeline,
        Kind::Sky => &sky_pipeline,
        Kind::Liquid => &liquid_pipeline,
    };
    let extra_properties = (kind == Kind::Liquid).then_some(&properties);
    let camera = Camera::create(&device);
    camera.write(&queue, camera::Data::construct(Vec3::ZERO, Vec2::ZERO, 1.0));
    let binding = bind_group::Camera::create(&device, &camera);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    let lightmap_array =
        crate::gpu::texture::Array::create(&device, [1; 2], atlases.len().max(1) as u32, 1);
    for (layer, pixels) in atlases.iter().enumerate() {
        lightmap_array.write(&queue, layer as u32, 0, bytemuck::cast_slice(pixels));
    }
    let lightmaps = bind_group::Array::create(&device, &lightmap_array, &sampler);
    let material = if kind == Kind::Sky {
        properties.bind_group()
    } else {
        lightmaps.bind_group()
    };
    let near = Color::create(&device, [2, 1], 1);
    near.write(&queue, 0, &[255, 0, 0, 255, 0, 255, 0, 255]);
    if kind == Kind::Sky {
        // Transparent foreground on the left must reveal the red background on the right.
        near.write(&queue, 0, &[0, 0, 0, 0, 255, 0, 0, 255]);
    }
    if kind == Kind::Liquid {
        near.write(&queue, 0, &[255, 0, 0, 255, 0, 0, 0, 255]);
    }
    let far = Color::create(&device, [1; 2], 1);
    far.write(&queue, 0, &[0, 0, 255, 255]);
    let textures = [
        bind_group::Color::create(&device, &far, &sampler),
        bind_group::Color::create(&device, &near, &sampler),
        bind_group::Color::create(&device, &far, &sampler),
    ];
    let draws = [
        (&textures[0], 3..6),
        (&textures[1], 0..3),
        (&textures[2], 3..6),
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
        for (texture, range) in &draws {
            pass.draw(
                selected_pipeline,
                &binding,
                &vertices,
                &indices,
                texture,
                material,
                extra_properties,
                range.clone(),
            );
        }
    }
    vertices.write(
        &queue,
        &[
            vertex::world::Data {
                position: [-768.0, 512.0, -768.0],
                uv: if kind == Kind::Liquid {
                    [0.25, 0.5]
                } else {
                    [2.25, 0.5]
                },
                lightmap_layers,
                light_uv: [[0.5; 2]; 4],
            },
            vertex::world::Data {
                position: [768.0, 512.0, -768.0],
                uv: if kind == Kind::Liquid {
                    [0.25, 0.5]
                } else {
                    [2.25, 0.5]
                },
                lightmap_layers,
                light_uv: [[0.5; 2]; 4],
            },
            vertex::world::Data {
                position: [0.0, 512.0, 768.0],
                uv: if kind == Kind::Liquid {
                    [0.25, 0.5]
                } else {
                    [2.25, 0.5]
                },
                lightmap_layers,
                light_uv: [[0.5; 2]; 4],
            },
            vertex::world::Data {
                position: [-1536.0, 1024.0, -1536.0],
                uv: if kind == Kind::Liquid {
                    [0.25, 0.5]
                } else {
                    [2.25, 0.5]
                },
                lightmap_layers,
                light_uv: [[0.5; 2]; 4],
            },
            vertex::world::Data {
                position: [1536.0, 1024.0, -1536.0],
                uv: if kind == Kind::Liquid {
                    [0.25, 0.5]
                } else {
                    [2.25, 0.5]
                },
                lightmap_layers,
                light_uv: [[0.5; 2]; 4],
            },
            vertex::world::Data {
                position: [0.0, 1024.0, 1536.0],
                uv: if kind == Kind::Liquid {
                    [0.25, 0.5]
                } else {
                    [2.25, 0.5]
                },
                lightmap_layers,
                light_uv: [[0.5; 2]; 4],
            },
        ],
    );
    // Switch from far to near, then redraw far to check both texture binding and depth.
    indices.write(&queue, &[0, 1, 2, 3, 4, 5]);
    {
        let mut pass = render_pass::World::create(&mut encoder, &view, depth.view());
        for (texture, range) in &draws {
            pass.draw(
                selected_pipeline,
                &binding,
                &vertices,
                &indices,
                texture,
                material,
                extra_properties,
                range.clone(),
            );
        }
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
    if let Some(expected_red) = expected_red {
        assert!(
            pixels[center].abs_diff(expected_red) <= 1,
            "red: {}, expected {expected_red}",
            pixels[center]
        );
    }
    assert_eq!(&pixels[center + 1..center + 3], &[0, 0]);
    assert_eq!(pixels[center + 3], 255);
    pixels[center]
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn indexed_world_binds_textures_per_range_and_occludes_farther_triangles() {
    assert_world_lighting(
        [vertex::world::MISSING_LIGHTMAP; 4],
        &[],
        Some(255),
        Kind::Solid,
        0.,
    );
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn samples_all_four_lightmap_array_layers_and_skips_missing_slots() {
    let atlases = [16, 32, 64, 8].map(|red| vec![[red, 0, 0, 255]]);
    assert_world_lighting([3, 2, 1, 0], &atlases, Some(240), Kind::Solid, 0.);
    assert_world_lighting(
        [
            0,
            vertex::world::MISSING_LIGHTMAP,
            2,
            vertex::world::MISSING_LIGHTMAP,
        ],
        &atlases,
        Some(160),
        Kind::Solid,
        0.,
    );
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn sky_composites_foreground_over_background_without_lightmapping() {
    assert_world_lighting([0; 4], &[vec![[0, 0, 0, 255]]], Some(255), Kind::Sky, 0.);
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn liquid_warp_changes_with_time_and_keeps_lightmaps_stationary() {
    let layers = [
        0,
        vertex::world::MISSING_LIGHTMAP,
        vertex::world::MISSING_LIGHTMAP,
        vertex::world::MISSING_LIGHTMAP,
    ];
    let atlases = [vec![[128, 0, 0, 255]]];
    let first = assert_world_lighting(layers, &atlases, None, Kind::Liquid, 0.);
    let second = assert_world_lighting(layers, &atlases, None, Kind::Liquid, std::f32::consts::PI);
    assert!(first.abs_diff(255) <= 1 || first == 0);
    assert!(second.abs_diff(255) <= 1 || second == 0);
    assert_ne!(
        first, second,
        "opposite phases should select different texels"
    );
}
