use crate::gpu::{bind_group, buffer::uniform, pipeline, render_pass};

const SHADER: &str = include_str!("../../../../shader/tint.wgsl");
const SIZE: u32 = 2;
const ROW_BYTES: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn tint_blends_direct_colour_values_and_resets_between_frames() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let buffer = uniform::Tint::create(&device);
    let binding = bind_group::Tint::create(&device, &buffer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let pipeline = pipeline::tint::create(&device, format, SHADER);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Tint test scene"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Tint test readback"),
        size: u64::from(ROW_BYTES * SIZE),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    for (label, scene, rgba, expected) in [
        (
            "partial overlay",
            wgpu::Color::RED,
            [0., 1., 0., 0.25],
            [191, 64, 0, 255],
        ),
        (
            "zero alpha",
            wgpu::Color::RED,
            [0., 1., 0., 0.],
            [255, 0, 0, 255],
        ),
        (
            "opaque overlay",
            wgpu::Color::RED,
            [0., 0., 1., 1.],
            [0, 0, 255, 255],
        ),
        (
            "leave water",
            wgpu::Color::RED,
            [0., 0., 1., 0.],
            [255, 0, 0, 255],
        ),
        (
            "gray",
            wgpu::Color::BLACK,
            [0.5, 0.5, 0.5, 1.],
            [128, 128, 128, 255],
        ),
        (
            "brown, opaque",
            wgpu::Color::BLACK,
            [130. / 255., 80. / 255., 50. / 255., 1.],
            [130, 80, 50, 255],
        ),
        (
            "brown, water strength",
            wgpu::Color::BLACK,
            [130. / 255., 80. / 255., 50. / 255., 128. / 255.],
            [65, 40, 25, 255],
        ),
        (
            "mid-gray scene, half black tint",
            wgpu::Color {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 1.,
            },
            [0., 0., 0., 0.5],
            [64, 64, 64, 255],
        ),
    ] {
        buffer.write(&queue, uniform::tint::Data::construct(rgba));
        let mut encoder = device.create_command_encoder(&Default::default());
        // Stand in for a rendered world; the tint pass must preserve this attachment.
        {
            let _scene = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(scene),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        {
            let mut pass = render_pass::Composite::create(&mut encoder, &view);
            pass.draw_tint(&pipeline, &binding);
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
        let pixels = readback.get_mapped_range(..).unwrap();
        for row in 0..SIZE {
            for column in 0..SIZE {
                let start = (row * ROW_BYTES + column * 4) as usize;
                for (&actual, expected) in pixels[start..start + 4].iter().zip(expected) {
                    assert!(
                        actual.abs_diff(expected) <= 1,
                        "{label}: {actual}, expected {expected}"
                    );
                }
            }
        }
        drop(pixels);
        readback.unmap();
    }
}
