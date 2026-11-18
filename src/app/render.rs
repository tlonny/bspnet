use super::{App, Graphics};
use bspnet::{bsp::Contents, gpu, level::state::State};

impl Graphics {
    /// Draw each brush with the pipeline matching its opacity.
    fn draw_brushes<'a>(&'a self, pass: &mut gpu::render_pass::World<'a>, transparent: bool) {
        for ((entity, opacity), state) in self
            .bsp
            .entities()
            .iter()
            .zip(&self.entity_opacity)
            .zip(&self.states)
        {
            let Some((model, alpha)) = entity.brush() else {
                continue;
            };
            if alpha == 0.0 || opacity.is_some() != transparent {
                continue;
            }
            let pipeline = if transparent {
                &self.transparent_solid_pipeline
            } else {
                &self.pipeline
            };
            let mesh = self.entity_meshes[model].as_ref().unwrap();
            let pressed = matches!(state, State::Button(button) if button.is_pressed());
            for batch in mesh.batches() {
                let texture_id = if pressed {
                    self.bsp
                        .texture_families()
                        .get(&batch.texture_id)
                        .and_then(|family| family.alternate.as_ref())
                        .and_then(|frames| frames.first())
                        .copied()
                        .unwrap_or(batch.texture_id)
                } else {
                    batch.texture_id
                };
                let texture = self.textures[texture_id].as_ref().unwrap();
                pass.draw(
                    pipeline,
                    &self.camera_binding,
                    mesh.vertices(),
                    mesh.indices(),
                    texture.binding(),
                    self.lightmaps.bind_group(),
                    opacity.as_ref(),
                    batch.indices.clone(),
                );
            }
        }
    }
}

impl App {
    /// Upload the current view, select its PVS, draw the world, and present the frame.
    pub fn render(&mut self) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        if graphics.config.width == 0 || graphics.config.height == 0 {
            return;
        }
        let (frame, suboptimal) = match graphics.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Lost => {
                graphics.surface = graphics
                    .instance
                    .create_surface(graphics.window.clone())
                    .expect("could not recreate window surface");
                graphics
                    .surface
                    .configure(&graphics.device, &graphics.config);
                graphics.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                graphics
                    .surface
                    .configure(&graphics.device, &graphics.config);
                graphics.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                graphics.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Validation => panic!("invalid surface frame"),
        };
        let elapsed = graphics.started.elapsed().as_secs_f32();
        graphics.properties_data.water.seconds = elapsed;
        graphics.properties_data.slime.seconds = elapsed;
        graphics
            .properties_buffer
            .write(&graphics.queue, graphics.properties_data.water);
        graphics
            .slime_properties_buffer
            .write(&graphics.queue, graphics.properties_data.slime);
        graphics.camera_buffer.write(
            &graphics.queue,
            gpu::buffer::uniform::camera::Data::construct(
                self.camera.position(),
                self.camera.rotation(),
                graphics.config.width as f32 / graphics.config.height as f32,
            ),
        );
        let mut tint = [0.; 4];
        if let Some(leaf) = graphics.bsp.leaf_at(self.camera.position().to_array()) {
            tint = match graphics.bsp.leaves()[leaf].contents {
                Contents::Water => graphics.properties_data.water.liquid_tint,
                Contents::Slime => graphics.properties_data.slime.liquid_tint,
                _ => [0.; 4],
            };
            if graphics.mesh.set_leaf(&graphics.queue, leaf).unwrap() {
                graphics
                    .window
                    .set_title(&format!("{} — leaf {leaf}", self.window_title));
            }
        }
        graphics.tint_buffer.write(
            &graphics.queue,
            gpu::buffer::uniform::tint::Data::construct(tint),
        );
        let color = frame.texture.create_view(&Default::default());
        let mut encoder = graphics.device.create_command_encoder(&Default::default());
        {
            let mut pass =
                gpu::render_pass::World::create(&mut encoder, &color, graphics.depth.view());
            graphics.mesh.draw_opaque(
                &mut pass,
                &graphics.pipeline,
                &graphics.camera_binding,
                &graphics.lightmaps,
                &graphics.textures,
                &graphics.sky_pipeline,
                &graphics.liquid_pipeline,
                &graphics.properties_binding,
                graphics.properties_data.water.liquid_alpha,
                &graphics.slime_properties_binding,
                graphics.properties_data.slime.liquid_alpha,
            );
            graphics.draw_brushes(&mut pass, false);
        }
        {
            let mut pass = gpu::render_pass::World::transparent(
                &mut encoder,
                &graphics.accumulation,
                &graphics.revealage,
                graphics.depth.view(),
            );
            graphics.mesh.draw_transparent(
                &mut pass,
                &graphics.transparent_liquid_pipeline,
                &graphics.camera_binding,
                &graphics.lightmaps,
                &graphics.textures,
                &graphics.properties_binding,
                graphics.properties_data.water.liquid_alpha,
                &graphics.slime_properties_binding,
                graphics.properties_data.slime.liquid_alpha,
            );
            graphics.draw_brushes(&mut pass, true);
        }
        {
            let mut pass = gpu::render_pass::Composite::create(&mut encoder, &color);
            pass.draw_transparency(&graphics.composite_pipeline, &graphics.composite_binding);
            if tint[3] > 0.0 {
                pass.draw_tint(&graphics.tint_pipeline, &graphics.tint_binding);
            }
        }
        graphics.queue.submit([encoder.finish()]);
        graphics.window.pre_present_notify();
        graphics.queue.present(frame);
        graphics.window.request_redraw();
        if suboptimal {
            graphics
                .surface
                .configure(&graphics.device, &graphics.config);
            graphics.window.request_redraw();
        }
    }
}
