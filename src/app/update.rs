use super::{App, SIM_STEP};

impl App {
    /// Advance the noclip camera by one fixed simulation step.
    pub fn update(&mut self) {
        self.camera.update(SIM_STEP.as_secs_f32());
        if let Some(bsp) = &self.bsp {
            let leaf = bsp
                .leaf_at(self.camera.position())
                .expect("could not locate camera leaf");
            if self.leaf != Some(leaf) {
                println!(
                    "Camera leaf: {leaf} (contents: {})",
                    bsp.leaves[leaf].contents
                );
                self.leaf = Some(leaf);
            }
        }
    }
}
