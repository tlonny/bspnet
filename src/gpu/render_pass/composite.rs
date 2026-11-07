use crate::gpu::bind_group;

const LABEL: &str = "Composite";
const TRANSPARENCY_GROUP: u32 = 0;
const TINT_GROUP: u32 = 0;
const TRIANGLE_VERTICES: u32 = 3;

pub struct Composite<'a> {
    pass: wgpu::RenderPass<'a>,
}

impl<'a> Composite<'a> {
    /// Load the rendered scene colour without attaching or modifying depth.
    pub fn create(encoder: &'a mut wgpu::CommandEncoder, color: &'a wgpu::TextureView) -> Self {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(LABEL),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        Self { pass }
    }

    /// Resolve transparency over the opaque scene before applying screen tint.
    pub fn draw_transparency(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        binding: &'a bind_group::Composite,
    ) {
        self.pass.set_pipeline(pipeline);
        self.pass
            .set_bind_group(TRANSPARENCY_GROUP, binding.bind_group(), &[]);
        self.pass.draw(0..TRIANGLE_VERTICES, 0..1);
    }

    /// Blend the configured tint over the existing scene using a fullscreen triangle.
    pub fn draw_tint(&mut self, pipeline: &'a wgpu::RenderPipeline, tint: &'a bind_group::Tint) {
        self.pass.set_pipeline(pipeline);
        self.pass.set_bind_group(TINT_GROUP, tint.bind_group(), &[]);
        self.pass.draw(0..TRIANGLE_VERTICES, 0..1);
    }
}
