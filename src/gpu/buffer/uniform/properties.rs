use std::{mem::size_of, num::NonZeroU64};

const BUFFER_LABEL: &str = "Level properties";
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Data {
    pub sky_foreground_speed: f32,
    pub sky_background_speed: f32,
    pub seconds: f32,
    pub liquid_alpha: f32,
    /// RGB tint followed by its screen-blend strength.
    pub liquid_tint: [f32; 4],
}

impl Data {
    /// Construct sky and liquid properties with animation starting at time zero.
    pub fn construct(
        sky_foreground_speed: f32,
        sky_background_speed: f32,
        liquid_alpha: f32,
        liquid_tint: [f32; 4],
    ) -> Self {
        Self {
            sky_foreground_speed,
            sky_background_speed,
            seconds: 0.,
            liquid_alpha,
            liquid_tint,
        }
    }
}

pub struct Properties {
    buffer: wgpu::Buffer,
}

impl Properties {
    /// Allocate the level rendering properties uniform.
    pub fn create(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(BUFFER_LABEL),
            size: size_of::<Data>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    /// Describe level rendering properties for the fragment shader.
    pub fn layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(size_of::<Data>() as u64),
            },
            count: None,
        }
    }

    /// Bind level rendering properties.
    pub fn binding_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.buffer.as_entire_binding(),
        }
    }

    /// Upload the level rendering properties.
    pub fn write(&self, queue: &wgpu::Queue, data: Data) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&data));
    }
}
