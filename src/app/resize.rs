use super::App;

impl App {
    /// Reconfigure the surface, suspending rendering for zero-sized windows.
    pub fn resize(&mut self, width: u32, height: u32) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        graphics.config.width = width;
        graphics.config.height = height;
        if width == 0 || height == 0 {
            return;
        }
        graphics
            .surface
            .configure(&graphics.device, &graphics.config);
        graphics.depth = bspnet::gpu::texture::Depth::create(&graphics.device, width, height);
        graphics.accumulation =
            bspnet::gpu::texture::Accumulation::create(&graphics.device, width, height);
        graphics.revealage =
            bspnet::gpu::texture::Revealage::create(&graphics.device, width, height);
        graphics.composite_binding = bspnet::gpu::bind_group::Composite::create(
            &graphics.device,
            &graphics.accumulation,
            &graphics.revealage,
        );
        graphics.window.request_redraw();
    }
}
