use crate::gpu::{
    bind_group,
    buffer::vertex,
    texture::{Accumulation, Depth, Revealage},
};

const LABEL: &str = "Solid transparent";
const VERTEX_ENTRY: &str = "vs_main";
const FRAGMENT_ENTRY: &str = "fs_transparent";

/// Create the transparent solid pipeline with lighting and OIT accumulation.
pub fn create(device: &wgpu::Device, source: &str) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(LABEL),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let camera_layout = bind_group::Camera::layout(device);
    let color_layout = bind_group::Color::layout(device);
    let lightmap_layout = bind_group::Array::layout(device);
    let opacity_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(LABEL),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<f32>() as u64),
            },
            count: None,
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(LABEL),
        bind_group_layouts: &[
            Some(&camera_layout),
            Some(&color_layout),
            Some(&lightmap_layout),
            Some(&opacity_layout),
        ],
        immediate_size: 0,
    });
    let transparent_targets = [
        Some(wgpu::ColorTargetState {
            format: Accumulation::format(),
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(wgpu::ColorTargetState {
            format: Revealage::format(),
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::RED,
        }),
    ];
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
            targets: &transparent_targets,
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: Depth::format(),
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
