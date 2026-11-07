const LABEL: &str = "Lightmap texture array";
const BYTES_PER_PIXEL: u32 = 4;

pub struct Array {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Array {
    /// Allocate a linear RGBA texture array with explicit layer and mip counts.
    pub fn create(
        device: &wgpu::Device,
        dimensions: [u32; 2],
        layers: u32,
        mip_levels: u32,
    ) -> Self {
        let [width, height] = dimensions;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(LABEL),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        Self { texture, view }
    }

    /// Upload one complete mip of a single array layer as tightly packed RGBA pixels.
    pub fn write(&self, queue: &wgpu::Queue, layer: u32, mip_level: u32, pixels: &[u8]) {
        assert!(
            layer < self.texture.depth_or_array_layers(),
            "layer outside texture array"
        );
        assert!(
            mip_level < self.texture.mip_level_count(),
            "mip level outside texture array"
        );
        let size = wgpu::Extent3d {
            width: (self.texture.width() >> mip_level).max(1),
            height: (self.texture.height() >> mip_level).max(1),
            depth_or_array_layers: 1,
        };
        assert_eq!(
            pixels.len(),
            size.width as usize * size.height as usize * BYTES_PER_PIXEL as usize,
            "pixel data must fill the texture array layer mip"
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * BYTES_PER_PIXEL),
                rows_per_image: Some(size.height),
            },
            size,
        );
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
