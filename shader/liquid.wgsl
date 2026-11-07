struct Camera {
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
}

struct Properties {
    sky_speeds: vec2<f32>,
    seconds: f32,
    liquid_alpha: f32,
    liquid_tint: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light_uv0: vec2<f32>,
    @location(3) light_uv1: vec2<f32>,
    @location(4) light_uv2: vec2<f32>,
    @location(5) light_uv3: vec2<f32>,
    @location(6) @interpolate(flat) lightmap_layers: vec4<u32>,
    @location(7) world_position: vec3<f32>,
}

const TAU: f32 = 6.28318530718;
const WARP_AMPLITUDE: f32 = 0.125;
const NOISE_SCALE: f32 = 1.0 / 192.0;
const NOISE_PHASE: f32 = 4.0;
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

@group(3) @binding(0) var<uniform> properties: Properties;

// Hash a noise lattice corner without making the pattern repeat at texture tile boundaries.
fn hash(cell: vec3<i32>) -> f32 {
    let bits = bitcast<vec3<u32>>(cell);
    var value = bits.x * 0x8da6b343u ^ bits.y * 0xd8163841u ^ bits.z * 0xcb1ab31fu;
    value = (value ^ (value >> 16u)) * 0x7feb352du;
    value = (value ^ (value >> 15u)) * 0x846ca68bu;
    value = value ^ (value >> 16u);
    return f32(value >> 8u) / 16777215.0;
}

// Interpolate world-space value noise with smooth first and second derivatives at cell edges.
fn noise(position: vec3<f32>) -> f32 {
    let origin = vec3<i32>(floor(position));
    let fraction = fract(position);
    let weight = fraction * fraction * fraction * (fraction * (fraction * 6.0 - 15.0) + 10.0);
    let near = mix(
        mix(hash(origin), hash(origin + vec3<i32>(1, 0, 0)), weight.x),
        mix(hash(origin + vec3<i32>(0, 1, 0)), hash(origin + vec3<i32>(1, 1, 0)), weight.x),
        weight.y,
    );
    let far = mix(
        mix(hash(origin + vec3<i32>(0, 0, 1)), hash(origin + vec3<i32>(1, 0, 1)), weight.x),
        mix(hash(origin + vec3<i32>(0, 1, 1)), hash(origin + vec3<i32>(1, 1, 1)), weight.x),
        weight.y,
    );
    return mix(near, far, weight.z);
}

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
    output.world_position = position;
    output.light_uv0 = light_uv0;
    output.light_uv1 = light_uv1;
    output.light_uv2 = light_uv2;
    output.light_uv3 = light_uv3;
    output.lightmap_layers = lightmap_layers;
    return output;
}

// Calculate warped and lit liquid colour for both rendering paths.
fn shade(input: VertexOutput) -> vec4<f32> {
    let point = input.world_position * NOISE_SCALE;
    let phase = NOISE_PHASE * (2.0 * vec2<f32>(noise(point), noise(point + vec3<f32>(37.7, 11.3, 57.9))) - 1.0);
    // Ironwail's base mapping, with smooth noise breaking up its regular wave phases.
    let warp = WARP_AMPLITUDE * sin(input.uv.yx * TAU + properties.seconds + phase);
    let color = textureSample(color_texture, color_sampler, input.uv * 2.0 + warp);
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

struct Transparency {
    @location(0) accumulation: vec4<f32>,
    @location(1) revealage: f32,
}

// Accumulate colour independently of draw order, favouring nearer fragments.
@fragment
fn fs_transparent(input: VertexOutput) -> Transparency {
    let color = shade(input);
    let alpha = clamp(properties.liquid_alpha, 0.0, 1.0);
    let weight = clamp(MAX_WEIGHT * pow(1.0 - input.position.z * DEPTH_WEIGHT, 3.0), MIN_WEIGHT, MAX_WEIGHT);
    var output: Transparency;
    output.accumulation = vec4<f32>(color.rgb * alpha, alpha) * weight;
    output.revealage = alpha;
    return output;
}
