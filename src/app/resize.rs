use bspnet::gpu::texture::Depth;

use super::App;

impl App {
    /// Recreate window attachments, suspending rendering for zero-sized windows.
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
        graphics.depth = Depth::create(&graphics.device, width, height);
        graphics.window.request_redraw();
    }
}
