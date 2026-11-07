use crate::gpu::{
    bind_group,
    buffer::{Index, vertex},
    texture,
};
use std::ops::Range;

const TRANSPARENT_LABEL: &str = "Transparent world";
const LABEL: &str = "World";
const CAMERA_GROUP: u32 = 0;
const COLOR_GROUP: u32 = 1;
const MATERIAL_GROUP: u32 = 2;
const PROPERTIES_GROUP: u32 = 3;
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

    /// Clear OIT attachments and test against opaque depth without changing it.
    pub fn transparent(
        encoder: &'a mut wgpu::CommandEncoder,
        accumulation: &'a texture::Accumulation,
        revealage: &'a texture::Revealage,
        depth: &'a wgpu::TextureView,
    ) -> Self {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(TRANSPARENT_LABEL),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: accumulation.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: revealage.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        Self { pass }
    }

    /// Bind the supplied resources and draw one range of triangle indices.
    pub fn draw(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        camera: &'a bind_group::Camera,
        vertices: &'a vertex::World,
        indices: &'a Index,
        texture: &'a bind_group::Color,
        material: &'a wgpu::BindGroup,
        properties: Option<&'a bind_group::Properties>,
        range: Range<u32>,
    ) {
        if indices.range().is_empty() || range.is_empty() {
            return;
        }
        self.pass.set_pipeline(pipeline);
        self.pass
            .set_bind_group(CAMERA_GROUP, camera.bind_group(), &[]);
        self.pass.set_bind_group(MATERIAL_GROUP, material, &[]);
        if let Some(properties) = properties {
            self.pass
                .set_bind_group(PROPERTIES_GROUP, properties.bind_group(), &[]);
        }
        self.pass
            .set_vertex_buffer(VERTEX_SLOT, vertices.buffer().slice(..));
        self.pass
            .set_index_buffer(indices.buffer().slice(..), Index::format());
        self.pass
            .set_bind_group(COLOR_GROUP, texture.bind_group(), &[]);
        self.pass.draw_indexed(range, 0, 0..1);
    }
}
