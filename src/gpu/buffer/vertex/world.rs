use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};

const BUFFER_LABEL: &str = "World vertices";
const POSITION_LOCATION: u32 = 0;
const UV_LOCATION: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Data {
    pub position: Vec3,
    pub uv: Vec2,
}

/// A fixed-capacity GPU buffer of textured world vertices.
pub struct World {
    buffer: wgpu::Buffer,
}

impl World {
    /// Allocate space for `capacity` world vertices.
    pub fn create(device: &wgpu::Device, capacity: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(BUFFER_LABEL),
            size: (capacity * std::mem::size_of::<Data>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    /// Upload vertex positions and UVs; wgpu validates capacity.
    pub fn write(&self, queue: &wgpu::Queue, vertices: &[Data]) {
        if !vertices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Describe the position and UV attributes.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Data>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::offset_of!(Data, position) as wgpu::BufferAddress,
                    shader_location: POSITION_LOCATION,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(Data, uv) as wgpu::BufferAddress,
                    shader_location: UV_LOCATION,
                },
            ],
        }
    }
}
