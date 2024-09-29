struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct Model {
    matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> model: Model;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = model.matrix * vec4<f32>(in.position, 1.0);
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

