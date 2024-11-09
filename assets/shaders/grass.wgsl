struct Instance {
    @location(1) pos: vec3<f32>,
    @location(2) hash: u32,
    @location(3) facing: vec2<f32>,
    @location(4) wind: f32,
    @location(5) pad: f32,
    @location(6) height: f32,
    @location(7) tilt: f32,
    @location(8) bend: f32,
    @location(9) width: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> debug_input: DebugInput;
@group(0) @binding(2) var<uniform> app_info: AppInfo;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
    facing: vec3<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

struct DebugInput { btn1: u32, btn2: u32, btn3: u32, btn4: u32, btn5: u32, btn6: u32, btn7: u32, btn8: u32, btn9: u32 };
fn btn1_pressed() -> bool { return debug_input.btn1 == 1u; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u; }
fn btn7_pressed() -> bool { return debug_input.btn7 == 1u; }
fn btn8_pressed() -> bool { return debug_input.btn8 == 1u; }
fn btn9_pressed() -> bool { return debug_input.btn9 == 1u; }

struct AppInfo {
    time_passed: f32
};

// grass
const HIGH_LOD = 14u;
const ANIM_FREQ = 3.0;
const ANIM_AMP = 0.1;
const ANIM_AMP_1 = 0.3;
const ANIM_AMP_2 = 0.4;
const ANIM_AMP_3 = 0.5;
const ANIM_OFFSET_1 = PI1_2 + PI1_8;
const ANIM_OFFSET_2 = PI1_2;
const ANIM_OFFSET_3 = 0.0;

const GLOBAL_WIND_MULT = 1.0;

const BEND_POINT_1 = 0.5;
const BEND_POINT_2 = 0.75;

const NORMAL_ROUNDING = PI / 6.0;
const SPECULAR_BLEND_MAX_DIST = 50.0;
const BASE_COLOR = vec3<f32>(0.05, 0.2, 0.01);
const TIP_COLOR = vec3<f32>(0.5, 0.5, 0.1);

// material
const AMBIENT_OCCLUSION = 1.0;
const ROUGHNESS = 0.5;
const METALNESS = 0.0;

const TERRAIN_NORMAL = vec3<f32>(0.0, 1.0, 0.0);

// constants
const PI = 3.1415927;
const PI1_2 = PI / 2.0;
const PI1_4 = PI / 4.0;
const PI1_8 = PI / 8.0;

@vertex
fn vs_main(
    instance: Instance,
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let facing = instance.facing;
    var height = instance.height;
    var tilt = instance.tilt;
    let bend = instance.bend;
    let wind = instance.wind;
    let hash = instance.hash;

    //tilt += wind * GLOBAL_WIND_MULT;
    //height -= wind * GLOBAL_WIND_MULT;

    let animation_offset = hash_to_range(hash, 0.0, 12.0 * PI);
    let t = (app_info.time_passed + animation_offset) * ANIM_FREQ;

    // Generate bezier curve
    let p0 = vec3<f32>(0.0, 0.0, 0.0);
    var p3 = vec3<f32>(tilt, height, tilt);
    var p1 = mix(p0, p3, BEND_POINT_1);
    var p2 = mix(p0, p3, BEND_POINT_2);

    let p1_bend = vec3<f32>((-tilt) * bend, abs(tilt) * bend, (-tilt) * bend);
    let p2_bend = vec3<f32>((-tilt) * bend, abs(tilt) * bend, (-tilt) * bend);
    let p1_wind = ANIM_AMP * ANIM_AMP_1 * vec3<f32>(cos(t + PI1_2 + ANIM_OFFSET_1), sin(t + ANIM_OFFSET_1), cos(t + PI1_2 + ANIM_OFFSET_1));
    let p2_wind = ANIM_AMP * ANIM_AMP_2 * vec3<f32>(cos(t + PI1_2 + ANIM_OFFSET_2), sin(t + ANIM_OFFSET_2), cos(t + PI1_2 + ANIM_OFFSET_2));
    let p3_wind = ANIM_AMP * ANIM_AMP_3 * vec3<f32>(cos(t + PI1_2 + ANIM_OFFSET_3), sin(t + ANIM_OFFSET_3), cos(t + PI1_2 + ANIM_OFFSET_3));

    // bend and wind
    p1 += p1_wind + p1_bend;
    p2 += p2_wind + p2_bend;
    p3 += p3_wind;

    // rotate towards facing
    p1 *= vec3<f32>(facing.x, 1.0, facing.y);
    p2 *= vec3<f32>(facing.x, 1.0, facing.y);
    p3 *= vec3<f32>(facing.x, 1.0, facing.y);

    // Generate vertex (High LOD)
    let p = f32(index / 2u * 2u) / f32(HIGH_LOD - 1u);
    var pos = bez(p, p0, p1, p2, p3);
    let dx = normalize(bez_dx(p, p0, p1, p2, p3));
    let orth = normalize(vec3<f32>(-instance.facing.y, 0.0, instance.facing.x));
    var normal = cross(dx, orth);

    // width and normal
    let width = mix(instance.width, 0.0, ease_in_cubic(p));
    pos = pos + orth * width * 0.5 * select(-1.0, 1.0, index % 2u == 0u);
    normal = normalize(normal + orth * NORMAL_ROUNDING * select(1.0, -1.0, index % 2u == 0u));

    let world_pos = instance.pos + pos;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.pos = world_pos;
    out.normal = normal;
    out.p = p;

    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) p: f32,
};

struct FragmentOutput {
    @location(0) position: vec4<f32>,
    @location(1) albedo: vec4<f32>,
    @location(2) normal: vec4<f32>,
    @location(3) roughness: vec4<f32>,
};

@fragment 
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool
) -> FragmentOutput {

    var normal = in.normal;
    if !front_facing {
        normal = -normal;
    }

    let dist_factor = saturate(length(camera.pos - in.pos) / SPECULAR_BLEND_MAX_DIST);
    normal = mix(normal, TERRAIN_NORMAL, ease_out(dist_factor));
    normal = (normal + 1.0) / 2.0; // [-1,1] -> [0,1]

    let roughness = ease_out(dist_factor) * 0.8;

    // interpolate color based of length
    let color = mix(BASE_COLOR, TIP_COLOR, ease_in(in.p)); // better interpolation function?

    var out: FragmentOutput;
    out.position = vec4<f32>(in.pos, 1.0);
    out.normal = vec4<f32>(normal, 1.0);
    out.albedo = vec4<f32>(color, 1.0);
    out.roughness = vec4<f32>(AMBIENT_OCCLUSION, roughness, METALNESS, 1.0); // ao, rough, metal, ?

    return out;
}


fn bez(p: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a * (pow(-p, 3.0) + 3.0 * pow(p, 2.0) - 3.0 * p + 1.0) + b * (3.0 * pow(p, 3.0) - 6.0 * pow(p, 2.0) + 3.0 * p) + c * (-3.0 * pow(p, 3.0) + 3.0 * pow(p, 2.0)) + d * (pow(p, 3.0));
}

fn bez_dx(p: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a * (-3.0 * pow(p, 2.0) + 6.0 * p - 3.0) + b * (9.0 * pow(p, 2.0) - 12.0 * p + 3.0) + c * (-9.0 * pow(p, 2.0) + 6.0 * p) + d * (3.0 * pow(p, 2.0));
}

//
// Easing functions
//

fn ease_in(p: f32) -> f32 {
    return p * p;
}

fn ease_in_cubic(p: f32) -> f32 {
    return p * p * p;
}

fn ease_out(p: f32) -> f32 {
    return 1.0 - pow(1.0 - p, 3.0);
}


//
// Hashing
//

fn hash_to_unorm(hash: u32) -> f32 {
    return f32(hash) * 2.3283064e-10; // hash * 1 / 2^32
}

fn hash_to_range(hash: u32, low: f32, high: f32) -> f32 {
    return low + (high - low) * hash_to_unorm(hash);
}

//fn rot_x(angle: f32) -> mat3x3<f32> {
//    let s = sin(angle);
//    let c = cos(angle);
//    return mat3x3<f32>(
//        1.0, 0.0, 0.0,
//        0.0, c, s,
//        0.0, -s, c,
//    );
//}
//fn rot_y(angle: f32) -> mat3x3<f32> {
//    let s = sin(angle);
//    let c = cos(angle);
//    return mat3x3<f32>(
//        c, 0.0, -s,
//        0.0, 1.0, 0.0,
//        s, 0.0, c,
//    );
//}
//fn rot_z(angle: f32) -> mat3x3<f32> {
//    let s = sin(angle);
//    let c = cos(angle);
//    return mat3x3<f32>(
//        c, s, 0.0,
//        -s, c, 0.0,
//        0.0, 0.0, 1.0,
//    );
//}
//
//// Function to calculate the inverse of a 3x3 matrix
//fn inverse_3x3(input_matrix: mat3x3<f32>) -> mat3x3<f32> {
//    // Calculate the determinant of the input matrix
//    let det = input_matrix[0][0] * (input_matrix[1][1] * input_matrix[2][2] - input_matrix[1][2] * input_matrix[2][1]) - input_matrix[0][1] * (input_matrix[1][0] * input_matrix[2][2] - input_matrix[1][2] * input_matrix[2][0]) + input_matrix[0][2] * (input_matrix[1][0] * input_matrix[2][1] - input_matrix[1][1] * input_matrix[2][0]);
//
//    // Calculate the inverse of the determinant
//    let invDet = 1.0 / det;
//
//    // Calculate the elements of the inverse matrix
//    var inverse_matrix: mat3x3<f32>;
//    inverse_matrix[0][0] = (input_matrix[1][1] * input_matrix[2][2] - input_matrix[1][2] * input_matrix[2][1]) * invDet;
//    inverse_matrix[0][1] = (input_matrix[0][2] * input_matrix[2][1] - input_matrix[0][1] * input_matrix[2][2]) * invDet;
//    inverse_matrix[0][2] = (input_matrix[0][1] * input_matrix[1][2] - input_matrix[0][2] * input_matrix[1][1]) * invDet;
//    inverse_matrix[1][0] = (input_matrix[1][2] * input_matrix[2][0] - input_matrix[1][0] * input_matrix[2][2]) * invDet;
//    inverse_matrix[1][1] = (input_matrix[0][0] * input_matrix[2][2] - input_matrix[0][2] * input_matrix[2][0]) * invDet;
//    inverse_matrix[1][2] = (input_matrix[0][2] * input_matrix[1][0] - input_matrix[0][0] * input_matrix[1][2]) * invDet;
//    inverse_matrix[2][0] = (input_matrix[1][0] * input_matrix[2][1] - input_matrix[1][1] * input_matrix[2][0]) * invDet;
//    inverse_matrix[2][1] = (input_matrix[0][1] * input_matrix[2][0] - input_matrix[0][0] * input_matrix[2][1]) * invDet;
//    inverse_matrix[2][2] = (input_matrix[0][0] * input_matrix[1][1] - input_matrix[0][1] * input_matrix[1][0]) * invDet;
//
//    return inverse_matrix;
//}

//const DEBUG_RED = vec4<f32>(1.0, 0.0, 0.0, 1.0);
//const DEBUG_IDENT_MAT = mat3x3<f32>(
//    1.0, 0.0, 0.0,
//    0.0, 1.0, 0.0,
//    0.0, 0.0, 1.0,
//);

    //let t = app_info.time_passed;
    //let light_pos = lights.main;
    ////let light_pos = debug_light_pos();
    //let light_dir = normalize(light_pos - in.pos);
    ////let light_dir = normalize(vec3<f32>(-1.0, 0.5, -1.0));
    //let view_dir = normalize(camera.pos - in.pos);

    //// Blend specular normal to terrain at distance
    //let dist_factor = saturate(length(camera.pos - in.pos) / SPECULAR_BLEND_MAX_DIST);
    //let specular_normal = mix(normal, TERRAIN_NORMAL, ease_out(dist_factor));
    //let reflect_dir = reflect(-light_dir, specular_normal);

    //// Only reflect on correct side
    //var specular = saturate(pow(dot(reflect_dir, view_dir), SPECULAR_INTENSITY));
    //if dot(normal, light_dir) <= 0.0 {
    //    specular *= ease_in(dist_factor); // fade as distance increases 
    //}
    //specular *= clamp(ease_out(1.0 - dist_factor), 0.7, 1.0);

    //// Phong
    //let ambient = 1.0;
    //let diffuse = saturate(dot(light_dir, normal));
    //var light = saturate(AMBIENT_MOD * ambient + DIFFUSE_MOD * diffuse + SPECULAR_MOD * specular);

    ////if btn1_pressed() { return vec4<f32>(normal.x, 0.0, normal.z, 1.0); }
    ////if btn2_pressed() { return vec4<f32>(specular, specular, specular, 1.0); }
    ////if btn3_pressed() { return vec4<f32>(diffuse, diffuse, diffuse, 1.0); }

    //let p = in.pos.y / 1.5;
    //let color = mix(BASE_COLOR, TIP_COLOR, ease_in(p)); // better interpolation function?

    ////return vec4<f32>(color * light, 1.0);
//fn debug_light_pos() -> vec3<f32> {
//    let t = app_info.time_passed;
//
//    var light_pos: vec3<f32>;
//    light_pos = vec3<f32>(15.0 + sin(t / 2.0) * 30.0, 6.0, 40.0);
//    light_pos = rotate_around(vec3<f32>(25.0, 10.0, 25.0), 30.0, t * 1.0);
//    light_pos = vec3<f32>(50.0, 16.0, -50.0);
//    return light_pos;
//}

//const LIGHT_ROTATION_SPEED = 0.5;
//fn rotate_around(center: vec3<f32>, radius: f32, time: f32) -> vec3<f32> {
//    return vec3<f32>(
//        center.x + radius * cos(time * LIGHT_ROTATION_SPEED),
//        center.y,
//        center.z + radius * sin(time * LIGHT_ROTATION_SPEED),
//    );
//}


    // branchless
    // tip
    //pos += select(0.0, 1.0, index == GRASS_MAX_VERT_INDEX) * dx * GRASS_TIP_EXTENSION;
    //// right
    //pos += select(0.0, 1.0, index % 2 == 0u) * orth * GRASS_WIDTH * 0.5;
    //normal += orth * NORMAL_ROUNDING;
    //// left
    //pos -= select(0.0, 1.0, index % 2 != 0u) * orth * GRASS_WIDTH * 0.5;
    //normal -= orth * NORMAL_ROUNDING;
    //normal = normalize(normal);

    //if index == GRASS_MAX_VERT_INDEX {
    //    //pos += dx * GRASS_TIP_EXTENSION;
    //// left
    //} else if index % 2u == 0u {
    //    pos += orth * width * 0.5;
    //    normal = normalize(normal - orth * NORMAL_ROUNDING);
    //// right
    //} else {
    //    pos -= orth * width * 0.5;
    //    normal = normalize(normal + orth * NORMAL_ROUNDING);
    //}
    //let sin1 = amplitude * sin(freq * (t + h));
    //let cos1 = amplitude * cos(freq * (t + h));
    //let sin2 = amplitude * sin(freq * (t + h + PI));
    //let cos2 = amplitude * cos(freq * (t + h + PI));
    //let local_wind = vec3<f32>(sin1 * WIND_TILT_MULT * facing.x, cos1 * WIND_HEIGHT_MULT, sin1 * WIND_TILT_MULT * facing.y);
    //let start = vec3<f32>(0.0, 0.0, 0.0);
    //let end = vec3<f32>(facing.x * tilt, height, facing.y * tilt) + local_wind;
    //let start_handle = mix(start, end, 0.6) + UP * bend * cos2 * WIND_BEND_BOT_MULT;
    //let end_handle = mix(start, end, 0.8) + UP * bend * cos2 * WIND_BEND_TOP_MULT;
    
    //let local_wind = vec3<f32>(sin1 * WIND_TILT_MULT * facing.x, cos1 * WIND_HEIGHT_MULT, sin1 * WIND_TILT_MULT * facing.y);

    //let start = vec3<f32>(0.0, 0.0, 0.0);
    //let end = vec3<f32>(facing.x * tilt, height, facing.y * tilt);
    //let start_handle = mix(start, end, 0.6) + UP * bend;
    //let end_handle = mix(start, end, 0.8) + UP * bend;
