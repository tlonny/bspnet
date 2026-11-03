use std::ops::Range;

const BUFFER_LABEL: &str = "Indices";
const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;

/// A fixed-capacity GPU buffer of 32-bit indices.
pub struct Index {
    buffer: wgpu::Buffer,
    count: u32,
}

impl Index {
    /// Allocate space for `capacity` indices.
    pub fn create(device: &wgpu::Device, capacity: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(BUFFER_LABEL),
            size: (capacity * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer, count: 0 }
    }

    /// Upload indices and update the draw range; `wgpu` validates capacity.
    pub fn write(&mut self, queue: &wgpu::Queue, indices: &[u32]) {
        if !indices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(indices));
        }
        self.count = u32::try_from(indices.len()).expect("too many world indices");
    }

    /// The uploaded indices to use for an indexed draw.
    pub fn range(&self) -> Range<u32> {
        0..self.count
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn format() -> wgpu::IndexFormat {
        INDEX_FORMAT
    }
}
