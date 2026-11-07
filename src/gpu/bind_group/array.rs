use crate::gpu::texture;

const LABEL: &str = "Texture array";
const TEXTURE_BINDING: u32 = 0;
const SAMPLER_BINDING: u32 = 1;

pub struct Array {
    bind_group: wgpu::BindGroup,
}

impl Array {
    /// Describe a sampled 2D texture array and sampler.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(LABEL),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: TEXTURE_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: SAMPLER_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Bind a texture array with a shared sampler.
    pub fn create(device: &wgpu::Device, array: &texture::Array, sampler: &wgpu::Sampler) -> Self {
        let layout = Self::layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(array.view()),
                },
                wgpu::BindGroupEntry {
                    binding: SAMPLER_BINDING,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Self { bind_group }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
