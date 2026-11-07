struct Camera {
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light_uv0: vec2<f32>,
    @location(3) light_uv1: vec2<f32>,
    @location(4) light_uv2: vec2<f32>,
    @location(5) light_uv3: vec2<f32>,
    @location(6) @interpolate(flat) lightmap_layers: vec4<u32>,
}

const MAX_WEIGHT: f32 = 1000.0;
const MIN_WEIGHT: f32 = 0.01;
const DEPTH_WEIGHT: f32 = 0.9;

const LIGHTMAP_SCALE: f32 = 2.0;
const MISSING_LIGHTMAP: u32 = 0xffffffffu;

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var color_texture: texture_2d<f32>;
@group(1) @binding(1) var color_sampler: sampler;
@group(2) @binding(0) var lightmaps: texture_2d_array<f32>;
@group(2) @binding(1) var lightmap_sampler: sampler;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light_uv0: vec2<f32>,
    @location(3) light_uv1: vec2<f32>,
    @location(4) light_uv2: vec2<f32>,
    @location(5) light_uv3: vec2<f32>,
    @location(6) lightmap_layers: vec4<u32>,
) -> VertexOutput {
    let view_position = camera.view * vec4<f32>(position, 1.0);
    var output: VertexOutput;
    output.position = camera.projection * view_position;
    output.uv = uv;
    output.light_uv0 = light_uv0;
    output.light_uv1 = light_uv1;
    output.light_uv2 = light_uv2;
    output.light_uv3 = light_uv3;
    output.lightmap_layers = lightmap_layers;
    return output;
}

// Shade a solid surface with its texture and baked lighting.
fn shade(input: VertexOutput) -> vec4<f32> {
    let color = textureSample(color_texture, color_sampler, input.uv);
    let uvs = array<vec2<f32>, 4>(input.light_uv0, input.light_uv1, input.light_uv2, input.light_uv3);
    var lighting = vec3<f32>(0.0);
    var lit = false;
    for (var slot = 0u; slot < 4u; slot += 1u) {
        let layer = input.lightmap_layers[slot];
        if (layer != MISSING_LIGHTMAP) {
            lighting += textureSampleLevel(lightmaps, lightmap_sampler, uvs[slot], i32(layer), 0.0).rgb;
            lit = true;
        }
    }
    if (lit) {
        lighting *= LIGHTMAP_SCALE;
    } else {
        lighting = vec3<f32>(1.0);
    }
    return vec4<f32>(clamp(color.rgb * lighting, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return shade(input);
}

@group(3) @binding(0) var<uniform> opacity: f32;

struct Transparency {
    @location(0) accumulation: vec4<f32>,
    @location(1) revealage: f32,
}

// Accumulate a translucent solid with the same weighting used by liquids.
@fragment
fn fs_transparent(input: VertexOutput) -> Transparency {
    let color = shade(input);
    let alpha = clamp(opacity, 0.0, 1.0);
    let weight = clamp(MAX_WEIGHT * pow(1.0 - input.position.z * DEPTH_WEIGHT, 3.0), MIN_WEIGHT, MAX_WEIGHT);
    return Transparency(vec4<f32>(color.rgb * alpha, alpha) * weight, alpha);
}
