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
    facing: vec3<f32>,
};

@group(1) @binding(0) var<uniform> app_info: AppInfo;
struct AppInfo {
    time_passed: f32,
};

@group(2) @binding(0) var<uniform> debug_input: DebugInput;
struct DebugInput { btn1: u32, btn2: u32, btn3: u32, btn4: u32, btn5: u32, btn6: u32, btn7: u32, btn8: u32, btn9: u32 };
fn btn1_pressed() -> bool { return debug_input.btn1 == 1u; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u; }

// grass
const GRASS_WIDTH = 0.1;
const GRASS_QUAD_AMOUNT = 4u;
const GRASS_MAX_VERT_INDEX = 14u;
const GRASS_BEND = 0.5;
const GRASS_TIP_EXTENSION = 0.1;

const NORMAL_ROUNDING = PI / 6.0;

const AMBIENT_MOD = 0.2;
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

    let facing = instance.facing * GRASS_BEND; // multiply to move further away
    let height = instance.height;

    // Generate vertex (High LOD)
    let t = f32(index / 2u * 2u) / f32(GRASS_MAX_VERT_INDEX);
    let a = vec3<f32>(0.0, 0.0, 0.0);
    let b = vec3<f32>(0.0, 1.0 * height, 0.0);
    let c = vec3<f32>(facing.x, 1.0 * height, facing.y);
    let d = vec3<f32>(facing.x, 1.0 * height, facing.y);
    let orth = vec3<f32>(-instance.facing.y, 0.0, instance.facing.x);

    var pos = bez(t, a, b, c, d);
    let dx = bez_dx(t, a, b, c, d);
    let normal = cross(dx, orth);
    var width_percent: f32;

    // tip
    if index == GRASS_MAX_VERT_INDEX {
        let dx = normalize(bez_dx(t - 1.0 / f32(GRASS_MAX_VERT_INDEX), a, b, c, d));
        pos += dx * GRASS_TIP_EXTENSION;
        width_percent = 0.5;
    // left
    } else if index % 2u == 0u {
        pos += orth * GRASS_WIDTH * 0.5;
        width_percent = 0.0;
    // right
    } else {
        pos -= orth * GRASS_WIDTH * 0.5;
        width_percent = 1.0;
    }

    // wind
    let wind_mat = rot_x(instance.wind.y) * rot_z(-instance.wind.x);

    var world_pos = instance.pos;
    // debug light pos
    //if instance_index == 2000u {
    //    world_pos = debug_light_pos();
    //}

    // model
    let rot_mat = wind_mat;
    let model_pos = world_pos + rot_mat * pos;

    // rounded normal
    let normal1 = transpose(inverse_3x3(rot_mat)) * normalize(normal + orth * NORMAL_ROUNDING);
    let normal2 = transpose(inverse_3x3(rot_mat)) * normalize(normal - orth * NORMAL_ROUNDING);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model_pos, 1.0);
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
    @location(1) normal1: vec3<f32>,
    @location(2) normal2: vec3<f32>,
    @location(3) width_percent: f32,
};

@fragment 
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool
) -> @location(0) vec4<f32> {
    // flip normals depending on face
    var normal: vec3<f32>;
    if front_facing {
        normal = mix(in.normal1, in.normal2, in.width_percent);
    } else {
        normal = mix(-in.normal2, -in.normal1, in.width_percent);
    }

    let t = app_info.time_passed;
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

    if btn1_pressed() { return vec4<f32>(normal.x, 0.0, normal.z, 1.0); }
    if btn2_pressed() { return vec4<f32>(specular, specular, specular, 1.0); }
    if btn3_pressed() { return vec4<f32>(diffuse, diffuse, diffuse, 1.0); }

    let p = in.pos.y / 1.5;
    let color = mix(BASE_COLOR, TIP_COLOR, ease_in(p)); // better interpolation function?

    return vec4<f32>(color * light, 1.0);
}

fn debug_light_pos() -> vec3<f32> {
    let t = app_info.time_passed;

    var light_pos: vec3<f32>;
    light_pos = vec3<f32>(15.0 + sin(t / 2.0) * 30.0, 6.0, 40.0);
    light_pos = rotate_around(vec3<f32>(25.0, 10.0, 25.0), 30.0, t * 1.0);
    light_pos = vec3<f32>(50.0, 16.0, -50.0);
    return light_pos;
}
const LIGHT_ROTATION_SPEED = 0.5;
fn rotate_around(center: vec3<f32>, radius: f32, time: f32) -> vec3<f32> {
    return vec3<f32>(
        center.x + radius * cos(time * LIGHT_ROTATION_SPEED),
        center.y,
        center.z + radius * sin(time * LIGHT_ROTATION_SPEED),
    );
}

//
// UTILS
//

fn bez(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a * (pow(-t, 3.0) + 3.0 * pow(t, 2.0) - 3.0 * t + 1.0) + b * (3.0 * pow(t, 3.0) - 6.0 * pow(t, 2.0) + 3.0 * t) + c * (-3.0 * pow(t, 3.0) + 3.0 * pow(t, 2.0)) + d * (pow(t, 3.0));
}

fn bez_dx(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a * (-3.0 * pow(t, 2.0) + 6.0 * t - 3.0) + b * (9.0 * pow(t, 2.0) - 12.0 * t + 3.0) + c * (-9.0 * pow(t, 2.0) + 6.0 * t) + d * (3.0 * pow(t, 2.0));
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
