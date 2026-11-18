use crate::gpu::{
    bind_group,
    buffer::{Index, uniform, vertex},
    pipeline, render_pass, texture,
};

const SIZE: u32 = 4;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const ROW_BYTES: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
const LIQUID_SHADER: &str = include_str!("../../../../shader/liquid.wgsl");
const SOLID_SHADER: &str = include_str!("../../../../shader/solid.wgsl");
const COMPOSITE_SHADER: &str = include_str!("../../../../shader/composite.wgsl");

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn transparency_is_order_independent_depth_tested_and_tinted_after_resolve() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let solid_transparent = pipeline::solid::transparent::create(&device, SOLID_SHADER);
    let liquid = pipeline::liquid::transparent::create(&device, LIQUID_SHADER);
    let world = pipeline::solid::opaque::create(&device, FORMAT, SOLID_SHADER);
    let composite = pipeline::composite::create(&device, FORMAT, COMPOSITE_SHADER);
    let tint_pipeline = pipeline::tint::create(
        &device,
        FORMAT,
        include_str!("../../../../shader/tint.wgsl"),
    );
    let accumulation = texture::Accumulation::create(&device, SIZE, SIZE);
    let revealage = texture::Revealage::create(&device, SIZE, SIZE);
    let composite_binding = bind_group::Composite::create(&device, &accumulation, &revealage);
    let depth = texture::Depth::create(&device, SIZE, SIZE);
    let camera = uniform::Camera::create(&device);
    camera.write(
        &queue,
        uniform::camera::Data::construct(glam::Vec3::ZERO, glam::Vec2::ZERO, 1.),
    );
    let camera_binding = bind_group::Camera::create(&device, &camera);
    let properties_buffer = uniform::Properties::create(&device);
    let properties = bind_group::Properties::create(&device, &properties_buffer);
    let tint_buffer = uniform::Tint::create(&device);
    let tint = bind_group::Tint::create(&device, &tint_buffer);
    let sampler = device.create_sampler(&Default::default());
    let colors = [[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]].map(|rgba| {
        let color = texture::Color::create(&device, [1; 2], 1);
        color.write(&queue, 0, &rgba);
        bind_group::Color::create(&device, &color, &sampler)
    });
    let atlas = texture::Array::create(&device, [1; 2], 1, 1);
    let lightmaps = bind_group::Array::create(&device, &atlas, &sampler);
    let vertices = vertex::World::create(&device, 12);
    // Each triangle covers the same screen region, at a different distance.
    let data: Vec<_> = [512., 1024., 768., 256.]
        .into_iter()
        .flat_map(|distance| {
            [[-1.5, -1.5], [1.5, -1.5], [0., 1.5]].map(|[x, z]| vertex::world::Data {
                position: [x * distance, distance, z * distance],
                uv: [0.5; 2],
                light_uv: [[0.; 2]; 4],
                lightmap_layers: [vertex::world::MISSING_LIGHTMAP; 4],
            })
        })
        .collect();
    vertices.write(&queue, &data);
    let mut indices = Index::create(&device, data.len());
    indices.write(&queue, &(0..data.len() as u32).collect::<Vec<_>>());
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Composite test scene"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Composite test readback"),
        size: u64::from(ROW_BYTES * SIZE),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut pixels = Vec::new();
    for (order, opaque, alpha, tint_rgba) in [
        (vec![0, 1], None, 0.5, [0.; 4]),
        (vec![1, 0], None, 0.5, [0.; 4]),
        (vec![0, 1], None, 0.5, [1., 0., 0., 0.5]),
        (vec![0, 1], Some(2), 0.5, [0.; 4]),
        (vec![0, 1], Some(3), 0.5, [0.; 4]),
        (vec![], None, 0.5, [0.; 4]),
        (vec![], None, 0.5, [1., 0., 0., 0.5]),
        (vec![0, 1], None, 0., [0.; 4]),
    ] {
        properties_buffer.write(
            &queue,
            uniform::properties::Data::construct(0., 0., alpha, [0.; 4]),
        );
        let opacity =
            pipeline::solid::transparent::opacity_binding(&device, &solid_transparent, alpha);
        tint_buffer.write(&queue, uniform::tint::Data::construct(tint_rgba));
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = render_pass::World::create(&mut encoder, &view, depth.view());
            if let Some(triangle) = opaque {
                pass.draw(
                    &world,
                    &camera_binding,
                    &vertices,
                    &indices,
                    &colors[2],
                    lightmaps.bind_group(),
                    None,
                    triangle * 3..triangle * 3 + 3,
                );
            }
        }
        {
            let mut pass = render_pass::World::transparent(
                &mut encoder,
                &accumulation,
                &revealage,
                depth.view(),
            );
            for triangle in order {
                let (pipeline, properties) = if triangle == 0 {
                    (&solid_transparent, &opacity)
                } else {
                    (&liquid, properties.bind_group())
                };
                pass.draw(
                    pipeline,
                    &camera_binding,
                    &vertices,
                    &indices,
                    &colors[triangle],
                    lightmaps.bind_group(),
                    Some(properties),
                    triangle as u32 * 3..triangle as u32 * 3 + 3,
                );
            }
        }
        {
            let mut pass = render_pass::Composite::create(&mut encoder, &view);
            pass.draw_transparency(&composite, &composite_binding);
            pass.draw_tint(&tint_pipeline, &tint);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ROW_BYTES),
                    rows_per_image: Some(SIZE),
                },
            },
            target.size(),
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
        let bytes = readback.get_mapped_range(..).unwrap();
        let center = (ROW_BYTES * (SIZE / 2) + 4 * (SIZE / 2)) as usize;
        pixels.push(<[u8; 4]>::try_from(&bytes[center..center + 4]).unwrap());
        drop(bytes);
        readback.unmap();
    }
    let [
        forward,
        reverse,
        tinted,
        occluded_far,
        occluded_both,
        empty,
        empty_tinted,
        zero_alpha,
    ] = pixels.as_slice()
    else {
        unreachable!()
    };
    for (a, b) in forward.iter().zip(reverse) {
        assert!(a.abs_diff(*b) <= 1, "draw order: {pixels:?}");
    }
    assert!(
        forward[0] > 80 && forward[2] > 80,
        "both liquid layers: {pixels:?}"
    );
    assert_eq!(forward[1], 0);
    for channel in 0..3 {
        let expected = (f32::from(forward[channel]) * 0.5 + if channel == 0 { 127.5 } else { 0. })
            .round() as u8;
        assert!(
            tinted[channel].abs_diff(expected) <= 1,
            "tint after transparency: {pixels:?}"
        );
    }
    assert!(
        occluded_far[0].abs_diff(128) <= 1 && occluded_far[1].abs_diff(128) <= 1,
        "opaque depth: {pixels:?}"
    );
    assert_eq!(occluded_far[2], 0);
    assert_eq!(*occluded_both, [0, 255, 0, 255]);
    assert_eq!(*empty, [0, 0, 0, 255]);
    assert_eq!(*empty_tinted, [128, 0, 0, 255]);
    assert_eq!(*zero_alpha, *empty);
}
