struct VertexInput {
    @builtin(instance_index) index: u32,
    @location(0) position: vec3f,
}

// @group(0) @binding(0) var<uniform> light_matrix: mat4x4f;
@group(0) @binding(0) var<storage, read> light_matrices: array<mat4x4f>;
@group(0) @binding(1) var<uniform> light_matrices_index: u32;
@group(0) @binding(2) var<storage, read> instances: array<Instance>;

struct Instance {
    transform: mat4x4f,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let model = instances[in.index].transform;
    var out: VertexOutput;
    out.clip_position = light_matrices[light_matrices_index] * model * vec4f(in.position, 1.0); // INFO: im only binding one and thus always take 0
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
}

@fragment
fn fs_main(in: VertexOutput) { }
