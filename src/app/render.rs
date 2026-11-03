use super::App;
use bspnet::gpu::{buffer::uniform::camera, render_pass};

impl App {
    /// Draw the complete indexed BSP mesh using the noclip camera.
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
        graphics.camera.write(
            &graphics.queue,
            camera::Data::construct(
                self.camera.position(),
                self.camera.rotation(),
                graphics.config.width as f32 / graphics.config.height as f32,
            ),
        );
        let color = frame.texture.create_view(&Default::default());
        let mut encoder = graphics.device.create_command_encoder(&Default::default());
        {
            let mut pass = render_pass::World::create(&mut encoder, &color, graphics.depth.view());
            pass.draw(
                &graphics.pipeline,
                &graphics.camera_binding,
                &graphics.vertices,
                &graphics.indices,
                &graphics.textures,
                &graphics.ranges,
            );
        }
        graphics.queue.submit([encoder.finish()]);
        graphics.window.pre_present_notify();
        graphics.queue.present(frame);
        if suboptimal {
            graphics
                .surface
                .configure(&graphics.device, &graphics.config);
            graphics.window.request_redraw();
        }
    }
}
