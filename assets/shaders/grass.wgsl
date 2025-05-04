struct Instance {
    @location(1) pos: vec3f,
    @location(2) hash: u32,
    @location(3) facing: vec2f,
    @location(4) wind: f32,
    @location(5) pad: f32,
    @location(6) height: f32,
    @location(7) tilt: f32,
    @location(8) bend: f32,
    @location(9) width: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> debug_input: DebugInput;
@group(0) @binding(2) var<uniform> app_info: AppInfo;

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

struct DebugInput {
    btn1: u32,
    btn2: u32,
    btn3: u32,
    btn4: u32,
    btn5: u32,
    btn6: u32,
    btn7: u32,
    btn8: u32,
    btn9: u32,
}

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
    time_passed: f32,
}

// grass
const HIGH_LOD = 15u;
const ANIM_FREQ = 3.0;
const ANIM_AMP = 0.2;
const ANIM_AMP_1 = ANIM_AMP * 0.3;
const ANIM_AMP_2 = ANIM_AMP * 0.4;
const ANIM_AMP_3 = ANIM_AMP * 0.5;
const ANIM_OFFSET_1 = PI1_2 + PI1_8;
const ANIM_OFFSET_2 = PI1_2;
const ANIM_OFFSET_3 = 0.0;

const WIND_DIR = vec2f(1.0, 1.0);
const GLOBAL_WIND_MULT = 1.0;
//const GLOBAL_WIND_FREQ_MULT = 0.10;

const BEND_POINT_1 = 0.5;
const BEND_POINT_2 = 0.75;

const NORMAL_ROUNDING = PI / 6.0;
const SPECULAR_BLEND_MAX_DIST = 50.0;
const BASE_COLOR = vec3f(0.05, 0.2, 0.01);
const TIP_COLOR = vec3f(0.5, 0.5, 0.1);

// material
const AMBIENT_OCCLUSION = 1.0;
const ROUGHNESS = 0.5;
const METALNESS = 0.0;

const TERRAIN_NORMAL = vec3f(0.0, 1.0, 0.0);

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

    let animation_offset = hash_to_range(hash, 0.0, 12.0 * PI);
    //let anim_freq = ANIM_FREQ * (height / MAX_HEIGHT);
    let anim_freq = ANIM_FREQ;
    let t = (app_info.time_passed + animation_offset) * anim_freq;

    // Generate bezier points
    let p0 = vec3f(0.0, 0.0, 0.0);
    var p3 = vec3f(tilt, height, tilt);
    var p1 = mix(p0, p3, BEND_POINT_1);
    var p2 = mix(p0, p3, BEND_POINT_2);

    // bend and wind
    let p1_bend = vec3f((-tilt) * bend, abs(tilt) * bend,(-tilt) * bend);
    let p2_bend = vec3f((-tilt) * bend, abs(tilt) * bend,(-tilt) * bend);
    let p1_wind = ANIM_AMP_1 * vec3f(cos(t + PI1_2 + ANIM_OFFSET_1), sin(t + ANIM_OFFSET_1), cos(t + PI1_2 + ANIM_OFFSET_1));
    let p2_wind = ANIM_AMP_2 * vec3f(cos(t + PI1_2 + ANIM_OFFSET_2), sin(t + ANIM_OFFSET_2), cos(t + PI1_2 + ANIM_OFFSET_2));
    let p3_wind = ANIM_AMP_3 * vec3f(cos(t + PI1_2 + ANIM_OFFSET_3), sin(t + ANIM_OFFSET_3), cos(t + PI1_2 + ANIM_OFFSET_3));
    p1 += p1_wind + p1_bend;
    p2 += p2_wind + p2_bend;
    p3 += p3_wind;

    // rotate towards facing
    p1 *= vec3f(facing.x, 1.0, facing.y);
    p2 *= vec3f(facing.x, 1.0, facing.y);
    p3 *= vec3f(facing.x, 1.0, facing.y);

    // Generate vertex (High LOD)
    let p = f32(index / 2u * 2u) / f32(HIGH_LOD - 1u);
    var pos = bez(p, p0, p1, p2, p3);
    let dx = normalize(bez_dx(p, p0, p1, p2, p3));
    let orth = normalize(vec3f(-instance.facing.y, 0.0, instance.facing.x));
    var normal = cross(dx, orth);

    // width and normal
    let width = mix(instance.width, 0.0, ease_in_cubic(p));
    pos = pos + orth * width * 0.5 * select(-1.0, 1.0, index % 2u == 0u);
    normal = normalize(normal + orth * NORMAL_ROUNDING * select(1.0, -1.0, index % 2u == 0u));

    let world_pos = instance.pos + pos;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4f(world_pos, 1.0);
    out.pos = world_pos;
    out.normal = normal;
    out.p = p;

    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
    @location(2) p: f32,
}

struct FragmentOutput {
    @location(0) position: vec4f,
    @location(1) albedo: vec4f,
    @location(2) normal: vec4f,
    @location(3) roughness: vec4f,
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> FragmentOutput {

    var normal = in.normal;
    if !front_facing {
        normal = -normal;
    }

    let dist_factor = saturate(length(camera.pos - in.pos) / SPECULAR_BLEND_MAX_DIST);
    // normal = mix(normal, TERRAIN_NORMAL, ease_out(dist_factor));
    // normal = (normal + 1.0) / 2.0; // [-1,1] -> [0,1]

    let roughness = ease_out(dist_factor) * 10.8;
    // let roughness = ()1.0 - smoothstep()
    // let roughness = 0.0;

    // interpolate color based of length
    let color = mix(BASE_COLOR, TIP_COLOR, ease_in(in.p)); // better interpolation function?

    var out: FragmentOutput;
    out.position = vec4f(in.pos, 1.0);
    out.normal = vec4f(normal, 1.0);
    out.albedo = vec4f(color, 1.0);
    out.roughness = vec4f(AMBIENT_OCCLUSION, roughness, METALNESS, 1.0); // ao, rough, metal, ?

    return out;
}

//
// Utils
//

fn bez(p: f32, a: vec3f, b: vec3f, c: vec3f, d: vec3f) -> vec3f {
    return a * (pow(-p, 3.0) + 3.0 * pow(p, 2.0) - 3.0 * p + 1.0) + b * (3.0 * pow(p, 3.0) - 6.0 * pow(p, 2.0) + 3.0 * p) + c * (-3.0 * pow(p, 3.0) + 3.0 * pow(p, 2.0)) + d * (pow(p, 3.0));
}

fn bez_dx(p: f32, a: vec3f, b: vec3f, c: vec3f, d: vec3f) -> vec3f {
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
} //fn rot_x(angle: f32) -> mat3x3f {
