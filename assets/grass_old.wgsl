struct Instance {
    @location(1) pos: vec3<f32>,
    @location(2) hash: u32,
    @location(3) facing: vec2<f32>,
    @location(4) wind: vec2<f32>,
    @location(5) pad: vec3<f32>,
    @location(6) height: f32,
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

// grass
const GRASS_WIDTH = 0.1;
const GRASS_QUAD_AMOUNT = 4u;
const GRASS_MAX_VERT_INDEX = 14u;
const GRASS_QUAD_HEIGHT = 1.0 / f32(GRASS_QUAD_AMOUNT);
const GRASS_MAX_ROT = PI / 8.0;

const NORMAL = vec3<f32>(0.0, 0.0, 1.0);
const NORMAL_ROUNDING = PI / 6.0;

const AMBIENT_MOD = 0.1;
const DIFFUSE_MOD = 0.5;
const SPECULAR_MOD = 2.0;
const SPECULAR_INTENSITY = 15.0; // must be odd
const SPECULAR_BLEND_MAX_DIST = 30.0;
const BASE_COLOR = vec3<f32>(0.05, 0.2, 0.01);
const TIP_COLOR = vec3<f32>(0.5, 0.5, 0.1);

const TERRAIN_NORMAL = vec3<f32>(0.0, 1.0, 0.0);

const PI = 3.1415927;
const X = vec3<f32>(1.0, 0.0, 0.0);
const Y = vec3<f32>(0.0, 1.0, 0.0);
const Z = vec3<f32>(0.0, 0.0, 1.0);

@vertex
fn vs_main(
    instance: Instance,
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {

    // Generate vertex (High LOD)
    var vpos = vec3<f32>(
        -GRASS_WIDTH * 0.5 + f32(index % 2u) * GRASS_WIDTH,
        GRASS_QUAD_HEIGHT * f32(index / 2u),
        0.0,
    );
    if index == GRASS_MAX_VERT_INDEX { vpos.x = 0.0; } // center last vertex
    // vpos.x += f32(index == GRASS_MAX_VERT_INDEX) * GRASS_WIDTH * 0.5; // non branching center last vertex

    // shape
    //let facing_angle = atan2(instance.facing.x, instance.facing.y); // x z
    let facing_angle = 0.0;
    let height_percent = vpos.y / instance.height;
    let shape_mat = rot_y(facing_angle) * rot_x(ease_in(height_percent) * GRASS_MAX_ROT);
   // let shape_mat = rot_y(facing_angle);

    // wind
    let wind_mat = rot_x(instance.wind.y) * rot_z(-instance.wind.x);

    var world_pos = instance.pos;
    // debug light pos
    if instance_index == 2000u {
        world_pos = debug_light_pos();
    }

    // model
    let rot_mat = wind_mat * shape_mat;
    let model_pos = world_pos + rot_mat * vpos;

    let normal = transpose(inverse_3x3(rot_mat)) * NORMAL;
    // rounded normal
    let normal1 = transpose(inverse_3x3(rot_y(-NORMAL_ROUNDING) * rot_mat)) * NORMAL;
    let normal2 = transpose(inverse_3x3(rot_y(NORMAL_ROUNDING) * rot_mat)) * NORMAL;
    let width_percent = (vpos.x + GRASS_WIDTH * 0.5) / GRASS_WIDTH;

    var out: VertexOutput;
    //out.clip_position = camera.view_proj * model_pos;
    out.clip_position = camera.view_proj * vec4<f32>(model_pos, 1.0);
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

fn debug_light_pos() -> vec3<f32> {
    let t = time_info.time_passed;

    var light_pos: vec3<f32>;
    light_pos = vec3<f32>(15.0 + sin(t / 2.0) * 30.0, 6.0, 40.0);
    light_pos = rotate_around(vec3<f32>(25.0, 10.0, 25.0), 30.0, t * 1.0);
    light_pos = vec3<f32>(50.0, 16.0, -50.0);
    return light_pos;
}

@fragment 
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool
) -> @location(0) vec4<f32> {

    // flip normals depending on face
    var normal: vec3<f32>;
    if front_facing {
        normal = in.normal;
        normal = mix(in.normal1, in.normal2, in.width_percent);
    } else {
        normal = -in.normal;
        normal = mix(-in.normal2, -in.normal1, in.width_percent);
    }

    let t = time_info.time_passed;
    let light_pos = debug_light_pos();
    let light_dir = normalize(light_pos - in.pos);
    //let light_dir = normalize(vec3<f32>(-1.0, 0.5, -1.0));
    let view_dir = normalize(camera.pos - in.pos);

    // Blend specular normal to terrain at distance
    let dist_factor = saturate(length(camera.pos - in.pos) / SPECULAR_BLEND_MAX_DIST);
    let specular_normal = mix(normal, TERRAIN_NORMAL, ease_out(dist_factor));
    let reflect_dir = reflect(-light_dir, specular_normal);

    // Only reflect on correct side
    var specular = saturate(pow(dot(reflect_dir, view_dir), SPECULAR_INTENSITY));
    if dot(normal, light_dir) <= 0.0 {
        specular *= ease_in(dist_factor); // fade as distance increases 
    }
    specular *= clamp(ease_out(1.0 - dist_factor), 0.7, 1.0);

    // Phong
    let ambient = 1.0;
    let diffuse = saturate(dot(light_dir, normal));
    var light = saturate(AMBIENT_MOD * ambient + DIFFUSE_MOD * diffuse + SPECULAR_MOD * specular);

    if btn_pressed() {
        var debug: vec4<f32>;
        debug = vec4<f32>(diffuse, diffuse, diffuse, 1.0);
        debug = vec4<f32>(reflect_dir, 1.0);
        debug = vec4<f32>(specular, specular, specular, 1.0);
        debug = vec4<f32>(normal, 1.0);
        debug = vec4<f32>(normal.x, 0.0, normal.z, 1.0);
        return debug;
    }

    let p = in.pos.y / 1.5;
    let color = mix(BASE_COLOR, TIP_COLOR, ease_in(p)); // better interpolation function?

    return vec4<f32>(color * light, 1.0);
}

//
// UTILS
//

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
        0.0, c, s,
        0.0, -s, c,
    );
}
fn rot_y(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        c, 0.0, -s,
        0.0, 1.0, 0.0,
        s, 0.0, c,
    );
}
fn rot_z(angle: f32) -> mat3x3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat3x3<f32>(
        c, s, 0.0,
        -s, c, 0.0,
        0.0, 0.0, 1.0,
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
