use std::io::{Error, ErrorKind, Result};

use crate::bsp::texture::{External, Indexed, Kind};
use crate::gpu::{bind_group, texture};

const OPAQUE_ALPHA: u8 = 255;

// Expand indexed mip levels while preserving the sky foreground mask.
fn expand_indexed(source: &Indexed, palette: &[[u8; 3]; 256]) -> Vec<Vec<[u8; 4]>> {
    source
        .mips
        .iter()
        .enumerate()
        .map(|(level, indices)| {
            let width = (source.dimensions[0] >> level).max(1) as usize;
            indices
                .iter()
                .enumerate()
                .map(|(pixel, &index)| {
                    if source.metadata.kind == Kind::Sky && pixel % width < width / 2 && index == 0
                    {
                        return [0; 4];
                    }
                    let [r, g, b] = palette[usize::from(index)];
                    [r, g, b, OPAQUE_ALPHA]
                })
                .collect()
        })
        .collect()
}

// Validate external pixels against BSP dimensions and expand them to opaque RGBA.
fn expand_rgb(source: &External, pixels: &[[u8; 3]]) -> Result<Vec<[u8; 4]>> {
    let dimensions = source.dimensions;
    let count = (dimensions[0] as usize).checked_mul(dimensions[1] as usize);
    if dimensions.contains(&0) || count != Some(pixels.len()) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "RGB pixels must fill nonzero texture dimensions",
        ));
    }
    Ok(pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, OPAQUE_ALPHA])
        .collect())
}

pub struct Texture {
    kind: Kind,
    binding: bind_group::Color,
}

impl Texture {
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn binding(&self) -> &bind_group::Color {
        &self.binding
    }
}

// Upload RGBA mips and bind the common repeat sampler.
fn upload(
    kind: Kind,
    dimensions: [u32; 2],
    mips: &[Vec<[u8; 4]>],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Texture {
    let color = texture::Color::create(device, dimensions, mips.len() as u32);
    for (level, pixels) in mips.iter().enumerate() {
        color.write(queue, level as u32, bytemuck::cast_slice(pixels));
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
        binding: bind_group::Color::create(device, &color, &sampler),
    }
}

/// Upload an indexed BSP texture using a decoded .lmp palette.
pub fn from_indexed(
    source: &Indexed,
    palette: &[[u8; 3]; 256],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Texture {
    let mips = expand_indexed(source, palette);
    upload(
        source.metadata.kind,
        source.dimensions,
        &mips,
        device,
        queue,
    )
}

/// Upload external RGB pixels as one opaque mip at the BSP texture dimensions.
pub fn from_rgb(
    source: &External,
    pixels: &[[u8; 3]],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Texture> {
    let pixels = expand_rgb(source, pixels)?;
    Ok(upload(
        source.metadata.kind,
        source.dimensions,
        &[pixels],
        device,
        queue,
    ))
}

#[cfg(test)]
mod test;
