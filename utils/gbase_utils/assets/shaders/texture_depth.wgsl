struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct Camera {
    pos: vec3<f32>,
    near: f32,
    facing: vec3<f32>,
    far: f32,

    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,

    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
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

@group(0) @binding(0) var tex: texture_depth_2d;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> camera: Camera;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureSample(tex, samp, in.uv);
    let linear_depth = linearize_depth_normalized(depth, camera.near, camera.far);

    return vec4f(linear_depth, linear_depth, linear_depth, 1.0);
}

//
// Utils
//

// Convert depth value to the actual depth value in range [near, far]
fn linearize_depth(depth: f32, near: f32, far: f32) -> f32 {
    return (2.0 * near * far) / (far + near - depth * (far - near));
}

// Convert depth value to the actual depth value in range [0, 1]
fn linearize_depth_normalized(depth: f32, near: f32, far: f32) -> f32 {
    return (2.0 * near * far) / (far + near - depth * (far - near)) / far;
}
