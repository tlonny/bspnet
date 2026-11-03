struct Camera {
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) depth: f32,
    @location(1) uv: vec2<f32>,
}

const DEPTH_RANGE: f32 = 2048.0;

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var color_texture: texture_2d<f32>;
@group(1) @binding(1)
var color_sampler: sampler;

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    let view_position = camera.view * vec4<f32>(position, 1.0);
    var output: VertexOutput;
    output.position = camera.projection * view_position;
    output.depth = -view_position.z;
    output.uv = uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let brightness = 1.0 - clamp(input.depth / DEPTH_RANGE, 0.0, 1.0);
    let color = textureSample(color_texture, color_sampler, input.uv);
    return vec4<f32>(color.rgb * brightness, color.a);
}
