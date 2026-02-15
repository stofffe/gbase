diagnostic (off, derivative_uniformity);

@group(0) @binding(0) var<uniform> camera: Camera;

struct Camera {
    pos: vec3f,
    near: f32,
    facing: vec3f,
    far: f32,

    view: mat4x4f,
    proj: mat4x4f,
    view_proj: mat4x4f,

    inv_view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view_proj: mat4x4f,
}

struct VertexInput {
    // instance
    @location(0) position: vec2f,
    @location(1) size: vec2f,
    @location(2) color: vec4f,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Generate quad corner 0..1
    let uv = vec2f(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );

    let pixel_pos = in.position + (uv) * in.size;
    let position = camera.view_proj * vec4f(pixel_pos, 0.0, 1.0);

    out.clip_position = position * vec4f(1.0,-1.0,1.0,1.0);
    out.uv = uv;
    out.color = in.color;

    return out;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // return in.color;
    return vec4f(in.uv, 0.0, 1.0);
}
