use crate::gpu::buffer::uniform;

const LABEL: &str = "Camera";
const CAMERA_BINDING: u32 = 0;

pub struct Camera {
    bind_group: wgpu::BindGroup,
}

impl Camera {
    /// Describe the camera uniform binding.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(LABEL),
            entries: &[uniform::Camera::layout_entry(CAMERA_BINDING)],
        })
    }

    /// Bind the camera uniform buffer.
    pub fn create(device: &wgpu::Device, camera: &uniform::Camera) -> Self {
        let layout = Self::layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &layout,
            entries: &[camera.binding_entry(CAMERA_BINDING)],
        });
        Self { bind_group }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
