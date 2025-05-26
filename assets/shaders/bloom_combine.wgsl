@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var bloom0: texture_2d<f32>;
@group(0) @binding(2) var bloom1: texture_2d<f32>;
@group(0) @binding(3) var bloom2: texture_2d<f32>;
@group(0) @binding(4) var bloom3: texture_2d<f32>;
@group(0) @binding(5) var bloom4: texture_2d<f32>;
@group(0) @binding(6) var samp: sampler;

// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
// TODO: NOTES
// dont extract? karis avg instead
// 13 tap
// 9 tap (write dir to output?)
// up should blur current level and add previous if exists
// blur 4 (only) =>
// blur 3 + blur from 4 =>
// blur 2 + blur from 3 =>
// blur 1 + blur from 2
// blur 0 + blur from 1

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let in_color = textureSample(tex, samp, in.uv);
    var bloom = textureSample(bloom0, samp, in.uv);
    let color = bloom + in_color;
    return vec4f(color);
}
