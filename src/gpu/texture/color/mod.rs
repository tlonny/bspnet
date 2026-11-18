const LABEL: &str = "Color texture";
const BYTES_PER_PIXEL: u32 = 4;

pub struct Color {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Color {
    /// Allocate an RGBA texture with the requested number of mip levels.
    pub fn create(device: &wgpu::Device, dimensions: [u32; 2], mip_levels: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(LABEL),
            size: wgpu::Extent3d {
                width: dimensions[0],
                height: dimensions[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view }
    }

    /// Upload one complete mip level of tightly packed RGBA pixels.
    pub fn write(&self, queue: &wgpu::Queue, mip_level: u32, pixels: &[u8]) {
        assert!(
            mip_level < self.texture.mip_level_count(),
            "mip level outside color texture"
        );
        let mut size = self.texture.size();
        size.width = (size.width >> mip_level).max(1);
        size.height = (size.height >> mip_level).max(1);
        assert_eq!(
            pixels.len(),
            size.width as usize * size.height as usize * BYTES_PER_PIXEL as usize,
            "pixel data must fill the color texture mip level"
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level,
                origin: wgpu::Origin3d::ZERO,
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

#[cfg(test)]
mod test;
