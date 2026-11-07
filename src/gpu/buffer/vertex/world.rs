use bytemuck::{Pod, Zeroable};

const BUFFER_LABEL: &str = "World vertices";
const POSITION_LOCATION: u32 = 0;
const UV_LOCATION: u32 = 1;
const LIGHT_UV_LOCATIONS: [u32; 4] = [2, 3, 4, 5];
const LIGHTMAP_LOCATION: u32 = 6;
const UV_SIZE: wgpu::BufferAddress = std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress;

/// Array-layer sentinel for a light style with no packed samples.
pub const MISSING_LIGHTMAP: u32 = u32::MAX;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Data {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub light_uv: [[f32; 2]; 4],
    pub lightmap_layers: [u32; 4],
}

impl Default for Data {
    fn default() -> Self {
        Self {
            position: [0.; 3],
            uv: [0.; 2],
            light_uv: [[0.; 2]; 4],
            lightmap_layers: [MISSING_LIGHTMAP; 4],
        }
    }
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

    /// Describe positions, base-texture UVs, and all four lightmap UV sets.
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
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(Data, light_uv) as wgpu::BufferAddress,
                    shader_location: LIGHT_UV_LOCATIONS[0],
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(Data, light_uv) as wgpu::BufferAddress + UV_SIZE,
                    shader_location: LIGHT_UV_LOCATIONS[1],
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(Data, light_uv) as wgpu::BufferAddress
                        + UV_SIZE * 2,
                    shader_location: LIGHT_UV_LOCATIONS[2],
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(Data, light_uv) as wgpu::BufferAddress
                        + UV_SIZE * 3,
                    shader_location: LIGHT_UV_LOCATIONS[3],
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32x4,
                    offset: std::mem::offset_of!(Data, lightmap_layers) as wgpu::BufferAddress,
                    shader_location: LIGHTMAP_LOCATION,
                },
            ],
        }
    }
}
