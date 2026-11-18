use std::ops::Range;

use crate::bsp::Data as Bsp;
use crate::gpu::{bind_group, texture};

use super::packer::Packer;

pub const ATLAS_SIZE: usize = 1024;
const TEXELS_PER_LUXEL: f64 = 16.0;
const STYLE_COUNT: usize = 4;
const OPAQUE_ALPHA: u8 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub style: u8,
    pub atlas: usize,
    /// Pixel origin of this rectangle in the atlas.
    pub position: [usize; 2],
}

#[derive(Debug)]
pub enum Face {
    Unlit,
    Lit {
        dimensions: [usize; 2],
        /// Project world positions into this face's local luxel coordinates.
        axes: [[f32; 4]; 2],
        references: [Option<Reference>; STYLE_COUNT],
    },
}

pub struct Builder<'a> {
    samples: &'a [[u8; 3]],
    packer: Packer,
    buffer: Vec<[u8; 4]>,
    atlases: Vec<Vec<[u8; 4]>>,
    occupied: bool,
}

struct Projection {
    dimensions: [usize; 2],
    axes: [[f32; 4]; 2],
    lighting_offset: usize,
}

// Resolve a usable lightmap projection and sample offset.
fn project(bsp: &Bsp, index: usize) -> Option<Projection> {
    let face = &bsp.faces()[index];
    let lighting_offset = face.lighting_offset?;
    let axes = bsp.texinfo()[face.texinfo].axes;
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for position in bsp.positions(index) {
        for axis in 0..2 {
            let [x, y, z, offset] = axes[axis];
            let value = f64::from(position[0] * x + position[1] * y + position[2] * z + offset);
            if !value.is_finite() {
                return None;
            }
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    let mut dimensions = [0; 2];
    let mut projection = [[0.0_f64; 4]; 2];
    for axis in 0..2 {
        let origin = (min[axis] / TEXELS_PER_LUXEL).floor();
        let size = (max[axis] / TEXELS_PER_LUXEL).ceil() - origin + 1.;
        if !size.is_finite() {
            return None;
        }
        if size < 1. {
            return None;
        }
        if size > ATLAS_SIZE as f64 {
            return None;
        }
        dimensions[axis] = size as usize;
        projection[axis] = axes[axis].map(|value| f64::from(value) / TEXELS_PER_LUXEL);
        projection[axis][3] -= origin;
    }
    let projection = Projection {
        dimensions,
        axes: projection.map(|axis| axis.map(|value| value as f32)),
        lighting_offset,
    };
    if projection
        .axes
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    Some(projection)
}

// Borrow a complete RGB style rectangle after checking its sample range.
fn pixels(samples: &[[u8; 3]], range: Range<usize>) -> Option<&[[u8; 3]]> {
    samples.get(range)
}

// Expand RGB samples to RGBA while copying rows into the packed atlas rectangle.
fn write(atlas: &mut [[u8; 4]], position: [usize; 2], dimensions: [usize; 2], pixels: &[[u8; 3]]) {
    let [width, height] = dimensions;
    for row in 0..height {
        let start = (position[1] + row) * ATLAS_SIZE + position[0];
        let destination = &mut atlas[start..start + width];
        let source = &pixels[row * width..(row + 1) * width];
        for (pixel, &[r, g, b]) in destination.iter_mut().zip(source) {
            *pixel = [r, g, b, OPAQUE_ALPHA];
        }
    }
}

impl<'a> Builder<'a> {
    /// Start packing lighting into fixed-size RGBA atlases.
    pub fn create(samples: &'a [[u8; 3]]) -> Self {
        Self {
            samples,
            packer: Packer::create([ATLAS_SIZE; 2]),
            buffer: vec![[0; 4]; ATLAS_SIZE * ATLAS_SIZE],
            atlases: Vec::new(),
            occupied: false,
        }
    }

    /// Pack one face's complete styles and return its temporary UV mapping.
    pub fn build(&mut self, bsp: &Bsp, index: usize) -> Face {
        let face = &bsp.faces()[index];
        let Some(Projection {
            dimensions,
            axes,
            lighting_offset,
        }) = project(bsp, index)
        else {
            return Face::Unlit;
        };
        let mut references = [None; STYLE_COUNT];
        let count = dimensions[0] * dimensions[1];
        for (slot, style) in face.styles.iter().enumerate() {
            let Some(style) = *style else {
                break;
            };
            let Some(start) = count
                .checked_mul(slot)
                .and_then(|skip| lighting_offset.checked_add(skip))
            else {
                continue;
            };
            let Some(end) = start.checked_add(count) else {
                continue;
            };
            let Some(pixels) = pixels(self.samples, start..end) else {
                continue;
            };
            let position = match self.packer.pack(dimensions) {
                Some(position) => position,
                None => {
                    self.atlases.push(std::mem::replace(
                        &mut self.buffer,
                        vec![[0; 4]; ATLAS_SIZE * ATLAS_SIZE],
                    ));
                    self.packer = Packer::create([ATLAS_SIZE; 2]);
                    self.packer.pack(dimensions).unwrap()
                }
            };
            write(&mut self.buffer, position, dimensions, pixels);
            self.occupied = true;
            references[slot] = Some(Reference {
                style,
                atlas: self.atlases.len(),
                position,
            });
        }
        if references.iter().any(Option::is_some) {
            Face::Lit {
                dimensions,
                axes,
                references,
            }
        } else {
            Face::Unlit
        }
    }

    /// Finish the final occupied page and return the RGBA atlas buffers.
    pub fn finish(mut self) -> Vec<Vec<[u8; 4]>> {
        if self.occupied {
            self.atlases.push(self.buffer);
        }
        self.atlases
    }

    /// Finish and upload all pages into one shared lightmap array binding.
    pub fn upload(self, device: &wgpu::Device, queue: &wgpu::Queue) -> bind_group::Array {
        let atlases = self.finish();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dimensions = if atlases.is_empty() {
            [1; 2]
        } else {
            [ATLAS_SIZE as u32; 2]
        };
        let layers = u32::try_from(atlases.len().max(1)).expect("too many lightmap array layers");
        let array = texture::Array::create(device, dimensions, layers, 1);
        for (layer, pixels) in atlases.into_iter().enumerate() {
            array.write(queue, layer as u32, 0, bytemuck::cast_slice(&pixels));
        }
        bind_group::Array::create(device, &array, &sampler)
    }
}
