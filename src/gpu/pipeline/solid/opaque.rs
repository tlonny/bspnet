use crate::gpu::{bind_group, buffer::vertex, texture::Depth};

const LABEL: &str = "Solid opaque";
const VERTEX_ENTRY: &str = "vs_main";
const FRAGMENT_ENTRY: &str = "fs_main";

/// Create the depth-tested opaque material pipeline.
pub fn create(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    source: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(LABEL),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let camera_layout = bind_group::Camera::layout(device);
    let color_layout = bind_group::Color::layout(device);
    let lightmap_layout = bind_group::Array::layout(device);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(LABEL),
        bind_group_layouts: &[
            Some(&camera_layout),
            Some(&color_layout),
            Some(&lightmap_layout),
        ],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(LABEL),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some(VERTEX_ENTRY),
            buffers: &[Some(vertex::World::layout())],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(FRAGMENT_ENTRY),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: Depth::format(),
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
