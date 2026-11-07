const LABEL: &str = "Transparency accumulation";

pub struct Accumulation {
    view: wgpu::TextureView,
}

impl Accumulation {
    pub fn format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba16Float
    }

    /// Allocate weighted colour and opacity storage at the render target's resolution.
    pub fn create(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(LABEL),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&Default::default()),
        }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
