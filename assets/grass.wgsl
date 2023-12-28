struct Instance {
    @location(1) pos: vec3<f32>,
    @location(2) hash: u32,
    @location(3) facing: vec2<f32>,
    @location(4) wind: f32,
    @location(5) pad: f32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

const GRASS_WIDTH = 0.1;
const GRASS_HEIGHT = 1.5;
const GRASS_QUAD_AMOUNT = 4u;
const GRASS_MAX_VERT_INDEX = 10u;
const GRASS_QUAD_HEIGHT = 1.0 / f32(GRASS_QUAD_AMOUNT);

const NORMAL = vec3<f32>(0.0, 0.0, 1.0);
const NORMAL_ROUNDING = 1.0 / 3.0;

const WIND_DIR = vec3<f32>(1.0, 0.0, 1.0); // TODO sample from texture instead

const PI = 3.1415927;

@vertex
fn vs_main(
    instance: Instance,
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    // Generate vertex (High LOD)
    var vpos = vec3<f32>(
        -GRASS_WIDTH * 0.5 + f32(index % 2u) * GRASS_WIDTH,
        GRASS_QUAD_HEIGHT * f32(index / 2u),
        0.0,
    );
    if index == GRASS_MAX_VERT_INDEX { vpos.x = 0.0; } // center last vertex
    // vpos.x += f32(index == GRASS_MAX_VERT_INDEX) * GRASS_WIDTH * 0.5; // non branching center last vertex

    // Shape
    let facing_angle = atan2(instance.facing.x, instance.facing.y); // x z
    let height_percent = vpos.y / GRASS_HEIGHT;
    let shape_mat = rot_y(PI - facing_angle) * rot_x(ease_in(height_percent) * PI / 8.);

    // Wind
    let wind_mat = rot_z(WIND_DIR.x * instance.wind) * rot_x(-WIND_DIR.z * instance.wind);

    // Apply pos and rot
    let rot_mat = wind_mat * shape_mat;
    var pos = vpos;
    pos = rot_mat * pos;        // rotate
    pos = instance.pos + pos;   // translate

    // normal
    let normal1 = rot_mat * rot_y(PI * NORMAL_ROUNDING) * NORMAL;
    let normal2 = rot_mat * rot_y(-PI * NORMAL_ROUNDING) * NORMAL;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.normal1 = normal1;
    out.normal2 = normal2;
    out.width_percent = (vpos.x + GRASS_WIDTH * 0.5) / GRASS_WIDTH;
    out.pos = pos;

    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>,
    @location(1) normal1: vec3<f32>,
    @location(2) normal2: vec3<f32>,
    @location(3) width_percent: f32,
};

const ambient_mod = 0.4;
const diffuse_mod = 0.7;
const base_color = vec3<f32>(0.05, 0.2, 0.01);
const tip_color = vec3<f32>(0.5, 0.5, 0.1);

@fragment 
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool
) -> @location(0) vec4<f32> {
    var normal = normalize(mix(in.normal1, in.normal2, in.width_percent)); // blend normals to get rounded normal

    // use if you want concave 
    // if front_facing { normal = -normal; }

    let light_pos = vec3<f32>(2.0, 1.0, -10.0);
    let light_dir = normalize(light_pos - in.pos);

    let diffuse = diffuse_mod * clamp(dot(light_dir, normal), 0.0, 1.0);
    let ambient = ambient_mod;
    let light = clamp(diffuse + ambient, 0.0, 1.0);

    let p = in.pos.y / 1.5;
    let color = mix(base_color, tip_color, ease_in(p)); // better interpolation function?

    // return vec4<f32>(normal.xz, 0.0, 1.0);
    return vec4<f32>(color * light, 1.0);
}

fn ease_in(p: f32) -> f32 {
    return p * p;
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
        c, -s, 0.0,
        s, c, 0.0,
        0.0, 0.0, 1.0
    );
}
// var<private> vertices: array<vec3<f32>, 11> = array<vec3<f32>, 11>(
//     vec3<f32>(-0.05, 0.0, 0.0),
//     vec3<f32>(0.05, 0.0, 0.0),
//     vec3<f32>(-0.05, 0.3, 0.0),
//     vec3<f32>(0.05, 0.3, 0.0),
//     vec3<f32>(-0.05, 0.6, 0.0),
//     vec3<f32>(0.05, 0.6, 0.0),
//     vec3<f32>(-0.05, 0.9, 0.0),
//     vec3<f32>(0.05, 0.9, 0.0),
//     vec3<f32>(-0.05, 1.2, 0.0),
//     vec3<f32>(0.05, 1.2, 0.0),
//     vec3<f32>(0.00, 1.5, 0.0),
// );
