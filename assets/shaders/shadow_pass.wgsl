struct VertexInput {
    @location(0) position: vec3f,
}

struct CameraUniform {
    pos: vec3f,
    facing: vec3f,

    view: mat4x4f,
    proj: mat4x4f,
    view_proj: mat4x4f,

    inv_view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view_proj: mat4x4f,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4f(model.position, 1.0);
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
}

// TODO: make empty
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(1.0, 1.0, 1.0, 1.0);
}
