struct Instance {
    @location(1) pos: vec3<f32>,
    @location(2) hash: u32,
    @location(3) facing: vec2<f32>,
    @location(4) wind: f32,
    @location(5) pad: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
    btn: u32,
};

// TODO DEBUG
fn btn_pressed() -> bool {
    return camera.btn == 1u;
}

@group(1) @binding(0) var<uniform> time_info: TimeInfo;
struct TimeInfo {
    time_passed: f32,
};

const GRASS_WIDTH = 0.1;
const GRASS_HEIGHT = 1.5;
const GRASS_QUAD_AMOUNT = 4u;
const GRASS_MAX_VERT_INDEX = 10u;
const GRASS_QUAD_HEIGHT = 1.0 / f32(GRASS_QUAD_AMOUNT);
const GRASS_MAX_ROT = PI / 8.0;

const NORMAL = vec3<f32>(0.0, 0.0, -1.0);
const NORMAL_ROUNDING = PI / 6.0;

const WIND_DIR = vec3<f32>(-1.0, 0.0, -1.0); // TODO sample from texture instead
const TERRAIN_NORMAL = vec3<f32>(0.0, 1.0, 0.0);

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

    let facing_angle = atan2(instance.facing.x, instance.facing.y); // x z
    let height_percent = vpos.y / GRASS_HEIGHT;
    let shape_mat = rot_x(ease_in(height_percent) * GRASS_MAX_ROT) * rot_y(facing_angle);
    let wind_mat = rot_z(-WIND_DIR.x * instance.wind) * rot_x(WIND_DIR.z * instance.wind);
    let rot_mat = shape_mat * wind_mat;
    //let rot_mat = rot_y(facing_angle);

    let model_matrix = transpose(mat4x4<f32>(
        rot_mat[0][0], rot_mat[0][1], rot_mat[0][2], instance.pos.x,
        rot_mat[1][0], rot_mat[1][1], rot_mat[1][2], instance.pos.y,
        rot_mat[2][0], rot_mat[2][1], rot_mat[2][2], instance.pos.z,
        0.0, 0.0, 0.0, 1.0,
    ));

    // normal
    let normal = transpose(inverse_3x3(rot_mat)) * NORMAL;

    // rounded normal
    let normal1 = transpose(inverse_3x3(rot_y(NORMAL_ROUNDING) * rot_mat)) * NORMAL;
    let normal2 = transpose(inverse_3x3(rot_y(-NORMAL_ROUNDING) * rot_mat)) * NORMAL;
    let width_percent = (vpos.x + GRASS_WIDTH * 0.5) / GRASS_WIDTH;

    //let normal = normal_matrix * NORMAL; // Apply pos and rot
    var out: VertexOutput;
    let model_pos = model_matrix * vec4<f32>(vpos, 1.0);

    out.clip_position = camera.view_proj * model_pos;
    out.normal = normal.xyz;
    out.normal1 = normal1.xyz;
    out.normal2 = normal2.xyz;
    out.width_percent = width_percent;
    out.pos = model_pos.xyz;

    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) normal1: vec3<f32>,
    @location(3) normal2: vec3<f32>,
    @location(4) width_percent: f32,
};

const AMBIENT_MOD = 0.1;
const DIFFUSE_MOD = 0.3;
const SPECULAR_MOD = 1.0;
const SPECULAR_INTENSITY = 30.0;
const BASE_COLOR = vec3<f32>(0.05, 0.2, 0.01);
const TIP_COLOR = vec3<f32>(0.5, 0.5, 0.1);
const SPECULAR_BLEND_MAX_DIST = 50.0;

@fragment 
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool
) -> @location(0) vec4<f32> {
    var normal: vec3<f32>;
    if front_facing {
        normal = mix(in.normal1, in.normal2, in.width_percent);
        // normal = in.normal;
    } else {
        normal = normalize(mix(-in.normal2, -in.normal1, in.width_percent));
        // normal = -in.normal;
    }

    let t = time_info.time_passed;
    let light_pos = rotate_around(vec3<f32>(5.0, 1.0, 5.0), 5.0, t * 2.0);
    //let light_pos = vec3<f32>(8.0, 3.0, 8.0);

    let light_dir = normalize(light_pos - in.pos);
    let view_dir = normalize(camera.pos - in.pos);

    let dist_factor = length(camera.pos - in.pos) / SPECULAR_BLEND_MAX_DIST;
    let specular_normal = mix(normal, TERRAIN_NORMAL, ease_in(dist_factor));
    let reflect_dir = reflect(-light_dir, specular_normal);

    // Phong
    let ambient = AMBIENT_MOD;
    let diffuse = DIFFUSE_MOD * saturate(dot(light_dir, normal));
    let specular = SPECULAR_MOD * saturate(pow(dot(reflect_dir, view_dir), SPECULAR_INTENSITY));
    let light = saturate(ambient + diffuse + specular);

    if btn_pressed() {
        //return vec4<f32>(normal.x, 0.0, normal.z, 1.0);
        return vec4<f32>(specular, specular, specular, 1.0);
    }

    let p = in.pos.y / 1.5;
    let color = mix(BASE_COLOR, TIP_COLOR, ease_in(p)); // better interpolation function?

    return vec4<f32>(color * light, 1.0);
}

const LIGHT_ROTATION_SPEED = 0.5;
fn rotate_around(center: vec3<f32>, radius: f32, time: f32) -> vec3<f32> {
    return vec3<f32>(
        center.x + radius * cos(time * LIGHT_ROTATION_SPEED),
        center.y,
        center.z + radius * sin(time * LIGHT_ROTATION_SPEED),
    );
}

fn ease_in(p: f32) -> f32 {
    return p * p;
}

fn ease_out(t: f32) -> f32 {
    return 1.0 - pow(1.0 - t, 3.0);
}

fn rot_x(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c, // s {transpose} -> -s {left handed} -> s
    );
}
fn rot_y(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        c, 0.0, s,// s {transpose} -> -s {left handed} -> s
        0.0, 1.0, 0.0,
        -s, 0.0, c,
    );
}
fn rot_z(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        c, -s, 0.0,
        s, c, 0.0, // s {transpose} -> -s {left handed} -> s
        0.0, 0.0, 1.0
    );
}

// Function to calculate the inverse of a 3x3 matrix
fn inverse_3x3(input_matrix: mat3x3<f32>) -> mat3x3<f32> {
    // Calculate the determinant of the input matrix
    let det = input_matrix[0][0] * (input_matrix[1][1] * input_matrix[2][2] - input_matrix[1][2] * input_matrix[2][1]) - input_matrix[0][1] * (input_matrix[1][0] * input_matrix[2][2] - input_matrix[1][2] * input_matrix[2][0]) + input_matrix[0][2] * (input_matrix[1][0] * input_matrix[2][1] - input_matrix[1][1] * input_matrix[2][0]);

    // Calculate the inverse of the determinant
    let invDet = 1.0 / det;

    // Calculate the elements of the inverse matrix
    var inverse_matrix: mat3x3<f32>;
    inverse_matrix[0][0] = (input_matrix[1][1] * input_matrix[2][2] - input_matrix[1][2] * input_matrix[2][1]) * invDet;
    inverse_matrix[0][1] = (input_matrix[0][2] * input_matrix[2][1] - input_matrix[0][1] * input_matrix[2][2]) * invDet;
    inverse_matrix[0][2] = (input_matrix[0][1] * input_matrix[1][2] - input_matrix[0][2] * input_matrix[1][1]) * invDet;
    inverse_matrix[1][0] = (input_matrix[1][2] * input_matrix[2][0] - input_matrix[1][0] * input_matrix[2][2]) * invDet;
    inverse_matrix[1][1] = (input_matrix[0][0] * input_matrix[2][2] - input_matrix[0][2] * input_matrix[2][0]) * invDet;
    inverse_matrix[1][2] = (input_matrix[0][2] * input_matrix[1][0] - input_matrix[0][0] * input_matrix[1][2]) * invDet;
    inverse_matrix[2][0] = (input_matrix[1][0] * input_matrix[2][1] - input_matrix[1][1] * input_matrix[2][0]) * invDet;
    inverse_matrix[2][1] = (input_matrix[0][1] * input_matrix[2][0] - input_matrix[0][0] * input_matrix[2][1]) * invDet;
    inverse_matrix[2][2] = (input_matrix[0][0] * input_matrix[1][1] - input_matrix[0][1] * input_matrix[1][0]) * invDet;

    return inverse_matrix;
}

const DEBUG_RED = vec4<f32>(1.0, 0.0, 0.0, 1.0);
const DEBUG_IDENT_MAT = mat3x3<f32>(
    1.0, 0.0, 0.0,
    0.0, 1.0, 0.0,
    0.0, 0.0, 1.0,
);


    //let model_matrix = mat4x4<f32>(
    //    rot_mat[0][0], rot_mat[1][0], rot_mat[2][0], 0.0,
    //    rot_mat[0][1], rot_mat[1][1], rot_mat[2][1], 0.0,
    //    rot_mat[0][2], rot_mat[1][2], rot_mat[2][2], 0.0,
    //    instance.pos.x, instance.pos.y, instance.pos.z, 1.0
    //);

    //let normal_matrix = transpose(inverse_3x3(mat3x3<f32>(
    //    model_matrix[0][0], model_matrix[0][1], model_matrix[0][2],
    //    model_matrix[1][0], model_matrix[1][1], model_matrix[1][2],
    //    model_matrix[2][0], model_matrix[2][1], model_matrix[2][2],
    //)));
