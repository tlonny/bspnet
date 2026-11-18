use crate::bsp::Data as Bsp;
use crate::gpu::buffer::vertex;
use crate::level::lightmap;

const LIQUID_UV_SCALE: f32 = 128.0;

/// Expand face corners and project base-texture and per-style atlas UVs.
pub fn build(bsp: &Bsp, index: usize, face: &lightmap::Face) -> Vec<vertex::world::Data> {
    let source = &bsp.faces()[index];
    let mapping = &bsp.texinfo()[source.texinfo];
    let texture = bsp.textures()[mapping.texture].as_ref().unwrap();
    let liquid = texture.kind().is_liquid();
    bsp.positions(index)
        .map(|position| {
            let uv = std::array::from_fn(|axis| {
                let [x, y, z, offset] = mapping.axes[axis];
                let projected = position[0] * x + position[1] * y + position[2] * z;
                if liquid {
                    projected / LIQUID_UV_SCALE
                } else {
                    (projected + offset) / texture.dimensions()[axis] as f32
                }
            });
            let (light_uv, lightmap_layers) = match face {
                lightmap::Face::Unlit => ([[0.; 2]; 4], [vertex::world::MISSING_LIGHTMAP; 4]),
                lightmap::Face::Lit {
                    axes, references, ..
                } => {
                    let local_uv = axes.map(|axis| {
                        let [x, y, z, offset] = axis.map(f64::from);
                        f64::from(position[0]) * x
                            + f64::from(position[1]) * y
                            + f64::from(position[2]) * z
                            + offset
                    });
                    let light_uv = references.map(|reference| match reference {
                        Some(reference) => std::array::from_fn(|axis| {
                            ((reference.position[axis] as f64 + local_uv[axis] + 0.5)
                                / lightmap::ATLAS_SIZE as f64) as f32
                        }),
                        None => [0.; 2],
                    });
                    let layers = references.map(|reference| {
                        reference.map_or(vertex::world::MISSING_LIGHTMAP, |reference| {
                            u32::try_from(reference.atlas).expect("too many lightmap layers")
                        })
                    });
                    (light_uv, layers)
                }
            };
            vertex::world::Data {
                position,
                uv,
                light_uv,
                lightmap_layers,
            }
        })
        .collect()
}
