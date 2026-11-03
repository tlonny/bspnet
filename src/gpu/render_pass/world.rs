use crate::gpu::{
    bind_group,
    buffer::{Index, vertex},
    pipeline,
};
use crate::level::TextureRange;

const LABEL: &str = "World";
const CAMERA_GROUP: u32 = 0;
const COLOR_GROUP: u32 = 1;
const VERTEX_SLOT: u32 = 0;

pub struct World<'a> {
    pass: wgpu::RenderPass<'a>,
}

impl<'a> World<'a> {
    /// Clear the window color and depth attachments.
    pub fn create(
        encoder: &'a mut wgpu::CommandEncoder,
        color: &'a wgpu::TextureView,
        depth: &'a wgpu::TextureView,
    ) -> Self {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(LABEL),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        Self { pass }
    }

    /// Draw ranges in order, binding a texture only when it changes.
    pub fn draw(
        &mut self,
        pipeline: &'a pipeline::World,
        camera: &'a bind_group::Camera,
        vertices: &'a vertex::World,
        indices: &'a Index,
        textures: &'a [bind_group::Color],
        ranges: &[TextureRange],
    ) {
        if indices.range().is_empty() {
            return;
        }
        self.pass.set_pipeline(pipeline.pipeline());
        self.pass
            .set_bind_group(CAMERA_GROUP, camera.bind_group(), &[]);
        self.pass
            .set_vertex_buffer(VERTEX_SLOT, vertices.buffer().slice(..));
        self.pass
            .set_index_buffer(indices.buffer().slice(..), Index::format());
        let mut bound_texture = None;
        for range in ranges {
            if bound_texture != Some(range.texture_id) {
                self.pass
                    .set_bind_group(COLOR_GROUP, textures[range.texture_id].bind_group(), &[]);
                bound_texture = Some(range.texture_id);
            }
            self.pass.draw_indexed(range.indices.clone(), 0, 0..1);
        }
    }
}
