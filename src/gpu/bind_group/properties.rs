use crate::gpu::buffer::uniform;

const LABEL: &str = "Level properties";
const SCROLLING_BINDING: u32 = 0;

pub struct Properties {
    bind_group: wgpu::BindGroup,
}

impl Properties {
    /// Describe the sky scrolling uniform.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(LABEL),
            entries: &[uniform::Properties::layout_entry(SCROLLING_BINDING)],
        })
    }

    /// Bind sky scrolling parameters shared by all sky textures.
    pub fn create(device: &wgpu::Device, sky: &uniform::Properties) -> Self {
        let layout = Self::layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &layout,
            entries: &[sky.binding_entry(SCROLLING_BINDING)],
        });
        Self { bind_group }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
