const TEXTURE_LABEL: &str = "World depth";

pub struct Depth {
    view: wgpu::TextureView,
}

impl Depth {
    pub fn format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Depth32Float
    }

    /// Allocate a depth attachment at the render target's resolution.
    pub fn create(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(TEXTURE_LABEL),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
