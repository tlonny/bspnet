const MIN_ACCUMULATED_ALPHA: f32 = 0.00001;

@group(0) @binding(1) var accumulation: texture_2d<f32>;
@group(0) @binding(2) var revealage: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[index], 0.0, 1.0);
}

// Resolve weighted transparent colour; alpha blending preserves the opaque scene beneath it.
@fragment
fn fs_transparency(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(position.xy);
    let remaining = textureLoad(revealage, pixel, 0).r;
    let sum = textureLoad(accumulation, pixel, 0);
    return vec4<f32>(sum.rgb / max(sum.a, MIN_ACCUMULATED_ALPHA), 1.0 - remaining);
}
