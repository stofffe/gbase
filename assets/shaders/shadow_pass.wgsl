struct VertexInput {
    @builtin(instance_index) index: u32,
    @location(0) position: vec3f,
}

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
    out.clip_position = light_matrix * model * vec4f(in.position, 1.0);
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
}

const BIAS = 0.005;
@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @builtin(frag_depth) f32 {
    var depth = in.clip_position.z;

    // if something is very thin the backface will be chosen
    // if something is thicker the fron will be chosen
    if front_facing {
        depth += BIAS;
    }

    return depth;
}
