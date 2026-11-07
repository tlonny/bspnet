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
    @location(0) direction: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sky_texture: texture_2d<f32>;
@group(2) @binding(0) var<uniform> properties: Properties;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    let view_position = camera.view * vec4<f32>(position, 1.0);
    let rotation = mat3x3<f32>(camera.view[0].xyz, camera.view[1].xyz, camera.view[2].xyz);
    var output: VertexOutput;
    output.position = camera.projection * view_position;
    output.direction = transpose(rotation) * view_position.xyz;
    return output;
}

// Wrap texels within one half, keeping foreground and background filtering independent.
fn texel(pixel: vec2<i32>, size: vec2<i32>, offset: i32) -> vec4<f32> {
    let wrapped = ((pixel % size) + size) % size;
    return textureLoad(sky_texture, wrapped + vec2<i32>(offset, 0), 0);
}

// Bilinearly sample a repeating half of the original packed sky texture.
fn layer(uv: vec2<f32>, foreground: bool) -> vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(sky_texture));
    let size = vec2<i32>(max(dimensions.x / 2, 1), dimensions.y);
    let offset = select(dimensions.x - size.x, 0, foreground);
    let pixel = fract(uv) * vec2<f32>(size) - 0.5;
    let origin = vec2<i32>(floor(pixel));
    let weight = fract(pixel);
    return mix(
        mix(texel(origin, size, offset), texel(origin + vec2<i32>(1, 0), size, offset), weight.x),
        mix(texel(origin + vec2<i32>(0, 1), size, offset), texel(origin + vec2<i32>(1, 1), size, offset), weight.x),
        weight.y,
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let direction = input.direction * vec3<f32>(1.0, 1.0, 3.0);
    let uv = direction.xy * (378.0 / 128.0) / max(length(direction), 0.000001);
    let scroll = properties.sky_speeds * properties.seconds / 128.0;
    let foreground = layer(uv + scroll.x, true);
    let background = layer(uv + scroll.y, false);
    // Transparent foreground texels contain zero RGB, so interpolation is premultiplied.
    return vec4<f32>(foreground.rgb + background.rgb * (1.0 - foreground.a), 1.0);
}
