use std::{mem::size_of, num::NonZeroU64};

use bytemuck::{Pod, Zeroable};
use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec2, Vec3};

const BUFFER_LABEL: &str = "Camera";
const FIELD_OF_VIEW: f32 = 75f32.to_radians();
const NEAR: f32 = 1.0;
const FAR: f32 = 16384.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Data {
    projection: Mat4,
    view: Mat4,
}

impl Data {
    /// Construct a Z-up camera from its position and (pitch, yaw) in radians.
    /// Zero rotation looks along +Y; projection uses right-handed, zero-to-one depth.
    pub fn construct(position: Vec3, rotation: Vec2, aspect_ratio: f32) -> Self {
        let (pitch_sin, pitch_cos) = rotation.x.sin_cos();
        let (yaw_sin, yaw_cos) = rotation.y.sin_cos();
        let forward = Vec3::new(yaw_sin * pitch_cos, yaw_cos * pitch_cos, pitch_sin);
        Self {
            projection: directx::perspective(FIELD_OF_VIEW, aspect_ratio, NEAR, FAR),
            view: view::look_to_mat4(position, forward, Vec3::Z),
        }
    }
}

pub struct Camera {
    buffer: wgpu::Buffer,
}

impl Camera {
    /// Allocate the camera's projection and view matrices.
    pub fn create(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(BUFFER_LABEL),
            size: size_of::<Data>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    /// Describe a camera uniform binding for the vertex shader.
    pub fn layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(size_of::<Data>() as u64),
            },
            count: None,
        }
    }

    /// Bind this camera's matrices.
    pub fn binding_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.buffer.as_entire_binding(),
        }
    }

    /// Upload the camera transforms for the next frame.
    pub fn write(&self, queue: &wgpu::Queue, data: Data) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&data));
    }
}
