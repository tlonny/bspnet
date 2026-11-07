use crate::bsp::{Data as Bsp, texture::Kind};
use crate::gpu::{bind_group, texture};

const SLIME_PREFIX: &str = "*slime";
const OPAQUE_ALPHA: u8 = 255;

pub struct Texture {
    kind: Kind,
    slime: bool,
    binding: bind_group::Color,
}

impl Texture {
    pub fn is_slime(&self) -> bool {
        self.slime
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn binding(&self) -> &bind_group::Color {
        &self.binding
    }
}

/// Upload one existing BSP texture's palette-expanded mip levels and return its binding.
pub fn build(
    bsp: &Bsp,
    index: usize,
    palette: &[[u8; 3]; 256],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Texture {
    let source = bsp.textures()[index].as_ref().expect("missing BSP texture");
    let kind = source.kind;
    let color = texture::Color::create(device, source.dimensions, source.mips.len() as u32);
    for (level, indices) in source.mips.iter().enumerate() {
        let width = (source.dimensions[0] >> level).max(1) as usize;
        let pixels: Vec<_> = indices
            .iter()
            .enumerate()
            .map(|(pixel, &index)| {
                if kind == Kind::Sky && pixel % width < width / 2 && index == 0 {
                    return [0; 4];
                }
                let [r, g, b] = palette[usize::from(index)];
                [r, g, b, OPAQUE_ALPHA]
            })
            .collect();
        color.write(queue, level as u32, bytemuck::cast_slice(&pixels));
    }
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    Texture {
        kind,
        slime: source
            .name
            .get(..SLIME_PREFIX.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SLIME_PREFIX)),
        binding: bind_group::Color::create(device, &color, &sampler),
    }
}
