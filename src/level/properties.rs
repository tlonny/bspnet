use crate::bsp::{Data as Bsp, Entity};
use crate::gpu::buffer::uniform::properties::Data as Uniform;

const DEFAULT_FOREGROUND_SPEED: f32 = 16.0;
const DEFAULT_BACKGROUND_SPEED: f32 = 8.0;
const DEFAULT_WATER_ALPHA: f32 = 0.5;
const DEFAULT_WATER_TINT: [f32; 3] = [130.0 / 255.0, 80.0 / 255.0, 50.0 / 255.0];
const DEFAULT_SLIME_ALPHA: f32 = 0.5;
const DEFAULT_SLIME_TINT: [f32; 3] = [0.0, 25.0 / 255.0, 5.0 / 255.0];
const DEFAULT_SLIME_TINT_STRENGTH: f32 = 150.0 / 255.0;
const DEFAULT_WATER_TINT_STRENGTH: f32 = 128.0 / 255.0;

pub struct Data {
    pub water: Uniform,
    pub slime: Uniform,
}

/// Resolve independent water and slime overrides with shared sky scrolling.
pub fn build(bsp: &Bsp) -> Data {
    let Entity::Worldspawn(world) = &bsp.entities()[0] else {
        unreachable!("BSP decoding requires worldspawn at entity zero");
    };
    let [tint_r, tint_g, tint_b] = world.water_tint.unwrap_or(DEFAULT_WATER_TINT);
    let water = Uniform::construct(
        world
            .sky_foreground_speed
            .unwrap_or(DEFAULT_FOREGROUND_SPEED),
        world
            .sky_background_speed
            .unwrap_or(DEFAULT_BACKGROUND_SPEED),
        world.water_alpha.unwrap_or(DEFAULT_WATER_ALPHA),
        [
            tint_r,
            tint_g,
            tint_b,
            world
                .water_tint_strength
                .unwrap_or(DEFAULT_WATER_TINT_STRENGTH),
        ],
    );
    let [r, g, b] = world.slime_tint.unwrap_or(DEFAULT_SLIME_TINT);
    let slime = Uniform {
        liquid_alpha: world.slime_alpha.unwrap_or(DEFAULT_SLIME_ALPHA),
        liquid_tint: [
            r,
            g,
            b,
            world
                .slime_tint_strength
                .unwrap_or(DEFAULT_SLIME_TINT_STRENGTH),
        ],
        ..water
    };
    Data { water, slime }
}
