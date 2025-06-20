struct VertexInput {
    @builtin(instance_index) index: u32,
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

// @group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(0) var<uniform> light_matrix: mat4x4f;
@group(0) @binding(1) var<storage, read> instances: array<Instance>;

struct Instance {
    transform: mat4x4f,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let model = instances[in.index].transform;
    var out: VertexOutput;
    // out.clip_position = camera.view_proj * model * vec4f(in.position, 1.0);
    out.clip_position = light_matrix * model * vec4f(in.position, 1.0);
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
}

// TODO: make empty
@fragment
fn fs_main(in: VertexOutput) {
}
