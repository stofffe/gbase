struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct Instance {
    @location(1) pos: vec3<f32>,
    @location(2) rot: vec2<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    in: VertexInput,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;
    let rot = instance.rot.x;
    let curve_amount = -instance.rot.y * in.position.y;
    let rotated_pos = rot_y(rot) * rot_z(curve_amount) * in.position;
    let pos = instance.pos + rotated_pos;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);

    //let normal = vec3<f32>(0.0, 0.0, 1.0);
    let rotated_normal = normalize(rot_y(rot) * rot_z(curve_amount) * normal);
    out.normal = rotated_normal;
    out.pos = pos;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

const normal = vec3<f32>(0.0, 0.0, 1.0);
const ambient_mod = 0.1;
const diffuse_mod = 0.7;
const color = vec3<f32>(0.2, 0.8, 0.2);

@fragment 
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool
) -> @location(0) vec4<f32> {
    var normal = in.normal;
    if front_facing {
        normal = -normal;
    }

    let light_pos = vec3<f32>(0.0, 1.0, -100.0);
    let light_dir = normalize(light_pos - in.pos);

    let diffuse = diffuse_mod * clamp(dot(light_dir, normal), 0.0, 1.0);
    let ambient = ambient_mod;
    let light = clamp(diffuse + ambient, 0.0, 1.0);

    return vec4<f32>(color * light, 1.0);
}

fn rot_x(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c,
    );
}
fn rot_y(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        c, 0.0, s,
        0.0, 1.0, 0.0,
        -s, 0.0, c,
    );
}
fn rot_z(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c,
    );
}
