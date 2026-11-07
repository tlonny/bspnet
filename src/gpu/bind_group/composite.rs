use crate::gpu::texture;

const LABEL: &str = "Transparency composite";
const ACCUMULATION_BINDING: u32 = 1;
const REVEALAGE_BINDING: u32 = 2;

pub struct Composite {
    bind_group: wgpu::BindGroup,
}

impl Composite {
    /// Describe the two unfiltered transparency attachments.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let entries =
            [ACCUMULATION_BINDING, REVEALAGE_BINDING].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(LABEL),
            entries: &entries,
        })
    }

    /// Bind the current screen-sized accumulation and revealage textures.
    pub fn create(
        device: &wgpu::Device,
        accumulation: &texture::Accumulation,
        revealage: &texture::Revealage,
    ) -> Self {
        let layout = Self::layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: ACCUMULATION_BINDING,
                    resource: wgpu::BindingResource::TextureView(accumulation.view()),
                },
                wgpu::BindGroupEntry {
                    binding: REVEALAGE_BINDING,
                    resource: wgpu::BindingResource::TextureView(revealage.view()),
                },
            ],
        });
        Self { bind_group }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
