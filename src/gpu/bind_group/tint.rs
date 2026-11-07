use crate::gpu::buffer::uniform;

const LABEL: &str = "Screen tint";
const TINT_BINDING: u32 = 0;

pub struct Tint {
    bind_group: wgpu::BindGroup,
}

impl Tint {
    /// Describe the screen tint uniform.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(LABEL),
            entries: &[uniform::Tint::layout_entry(TINT_BINDING)],
        })
    }

    /// Bind screen tint colour and strength.
    pub fn create(device: &wgpu::Device, tint: &uniform::Tint) -> Self {
        let layout = Self::layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &layout,
            entries: &[tint.binding_entry(TINT_BINDING)],
        });
        Self { bind_group }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
