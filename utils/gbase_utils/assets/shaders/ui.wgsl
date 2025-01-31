struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) @interpolate(flat) ty: u32, // 0 shape 1 text
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
}
;

const TYPE_SHAPE = 0u;
const TYPE_TEXT = 1u;

@group(0) @binding(0) var letter_tex: texture_2d<f32>;
@group(0) @binding(1) var letter_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: CameraUniform;

struct CameraUniform {
    pos: vec3<f32>,
    facing: vec3<f32>,

    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,

    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}
;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.uv = in.uv;
    out.ty = in.ty;

    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) @interpolate(flat) ty: u32,
    @location(2) uv: vec2<f32>,
}
;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Have to sample outside
    let alpha = textureSample(letter_tex, letter_sampler, in.uv).x;

    if in.ty == TYPE_TEXT {
        return vec4<f32>(in.color.xyz, alpha);
    }
    return in.color;
}
