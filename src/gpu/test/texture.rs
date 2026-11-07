use crate::gpu::texture::Color;

const SHADER: &str = r#"
const TARGET_MIP: u32 = 0u;
@group(0) @binding(0) var tex_sampler: sampler;
@group(0) @binding(1) var colors: texture_2d<f32>;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2<f32>(-1, -1), vec2<f32>(3, -1), vec2<f32>(-1, 3));
    return vec4<f32>(positions[index], 0, 1);
}

@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(position.x, 0.5) / vec2<f32>(textureDimensions(colors, TARGET_MIP));
    return textureSampleLevel(colors, tex_sampler, uv, f32(TARGET_MIP));
}
"#;
const REPEATED_RED_GREEN: [u8; 16] = [
    255, 0, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255,
];
const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const OUTPUT_SIZE: wgpu::Extent3d = wgpu::Extent3d {
    width: 4,
    height: 1,
    depth_or_array_layers: 1,
};

// Upload all supplied mips and verify sampling the selected mip with repeating UVs.
fn assert_sampled_mip(dimensions: [u32; 2], mips: &[&[u8]], target_mip: u32, expected: &[u8; 16]) {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let color = Color::create(&device, dimensions, mips.len() as u32);
    for (level, pixels) in mips.iter().enumerate() {
        color.write(&queue, level as u32, pixels);
    }
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            SHADER
                .replace(
                    "const TARGET_MIP: u32 = 0u;",
                    &format!("const TARGET_MIP: u32 = {target_mip}u;"),
                )
                .into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(OUTPUT_FORMAT.into())],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(color.view()),
            },
        ],
    });
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: OUTPUT_SIZE,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OUTPUT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &binding, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(1),
            },
        },
        OUTPUT_SIZE,
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
    assert_eq!(&pixels[..16], expected);
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn uploads_color_texture_and_samples_repeating_uvs() {
    assert_sampled_mip(
        [2, 1],
        &[&[255, 0, 0, 255, 0, 255, 0, 255]],
        0,
        &REPEATED_RED_GREEN,
    );
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn uploads_and_samples_a_specific_mip_level() {
    let base = [0, 0, 255, 255].repeat(8);
    let mip = [255, 0, 0, 255, 0, 255, 0, 255];
    assert_sampled_mip([4, 2], &[&base, &mip, &[255; 4]], 1, &REPEATED_RED_GREEN);
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn samples_palette_colour_values_unchanged() {
    assert_sampled_mip(
        [2, 1],
        &[&[130, 80, 50, 255, 64, 128, 192, 255]],
        0,
        &[
            130, 80, 50, 255, 64, 128, 192, 255, 130, 80, 50, 255, 64, 128, 192, 255,
        ],
    );
}
