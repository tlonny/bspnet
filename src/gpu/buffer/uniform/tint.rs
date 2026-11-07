use std::{mem::size_of, num::NonZeroU64};

const BUFFER_LABEL: &str = "Screen tint";

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Data {
    rgba: [f32; 4],
}

impl Data {
    /// Package the supplied tint unchanged, without colour-space conversion.
    pub fn construct(rgba: [f32; 4]) -> Self {
        Self { rgba }
    }
}

pub struct Tint {
    buffer: wgpu::Buffer,
}

impl Tint {
    /// Allocate the screen tint uniform.
    pub fn create(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(BUFFER_LABEL),
            size: size_of::<Data>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    /// Describe screen tint for the fragment shader.
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

    /// Bind screen tint.
    pub fn binding_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.buffer.as_entire_binding(),
        }
    }

    /// Upload the screen tint.
    pub fn write(&self, queue: &wgpu::Queue, data: Data) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&data));
    }
}
