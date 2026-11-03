use glam::UVec2;

const LABEL: &str = "Color texture";
const BYTES_PER_PIXEL: u32 = 4;

pub struct Color {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Color {
    /// Allocate an sRGB RGBA texture with one mip level.
    pub fn create(device: &wgpu::Device, dimensions: UVec2) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(LABEL),
            size: wgpu::Extent3d {
                width: dimensions.x,
                height: dimensions.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view }
    }

    /// Upload a complete texture of tightly packed RGBA pixels.
    pub fn write(&self, queue: &wgpu::Queue, pixels: &[u8]) {
        let size = self.texture.size();
        assert_eq!(
            pixels.len(),
            size.width as usize * size.height as usize * BYTES_PER_PIXEL as usize,
            "pixel data must fill the color texture"
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
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
