// Vertex

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> FragmentInput {
    var out: FragmentInput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var position_tex: texture_2d<f32>;
@group(0) @binding(2) var albedo_tex: texture_2d<f32>;
@group(0) @binding(3) var normal_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<f32>;
@group(0) @binding(5) var<uniform> camera: Camera;
@group(0) @binding(6) var<uniform> light: vec3<f32>;
@group(0) @binding(7) var<uniform> debug_input: DebugInput;

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

struct Camera {
    pos: vec3f,
    facing: vec3f,

    view: mat4x4f,
    proj: mat4x4f,
    view_proj: mat4x4f,

    inv_view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view_proj: mat4x4f,
}


struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

const SPECULAR_INTENSITY = 150.0;
const SPECULAR_MODIFIER = 2.7;
const SPECULAR_DITHER = 0.3;

const DIFFUSE_MODIFIER = 0.2;
const DIFFUSE_DITHER = 0.0;

const AMBIENT_MODIFIER = 0.15;
const AMBIENT_DITHER = 0.15;

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // Gather g-buffer data
    let position = textureSample(position_tex, samp, in.uv).xyz;
    let albedo = textureSample(albedo_tex, samp, in.uv).xyz;
    var normal = textureSample(normal_tex, samp, in.uv).xyz;
    // normal = normalize(normal * 2.0 - 1.0); // [0,1] -> [-1,1]
    let ambient_occlusion = textureSample(roughness_tex, samp, in.uv).r;
    let roughness = textureSample(roughness_tex, samp, in.uv).g; // Invert so higher => more relfection
    let metalness = textureSample(roughness_tex, samp, in.uv).b; // 0 = no metal, 1 = full metal

    let color = pbr_lighting(
        normal,
        camera.pos - position,
        light - position,
        vec3f(1.0),
        albedo,
        vec3f(0.0),
        roughness,
        metalness,
        ambient_occlusion,
    );

    // Debug
    if btn1_pressed() {
        return vec4<f32>(albedo, 1.0);
    }
    if btn2_pressed() {
        return vec4<f32>(normal, 1.0);
    }
    if btn3_pressed() {
        return vec4<f32>(position, 1.0);
    }
    return vec4f(color, 1.0);

// // Phong shading
// let light_dir = normalize(light - position);
// // let light_dir = normalize(vec3f(-10.0, 1.0, 10.0));
// let view_dir = normalize(camera.pos - position);
// let half_dir = normalize(light_dir + view_dir);
//
// let dither = rand(in.clip_position.xy) - 0.5;
//
// let ambient = AMBIENT_MODIFIER;
// let ambient_light = ambient + dither * AMBIENT_DITHER * ambient;
// // let ambient_light = 0.0;
//
// let diffuse = DIFFUSE_MODIFIER * saturate(dot(normal, light_dir)) * (1.0 - metalness);
// let diffuse_light = diffuse + dither * DIFFUSE_DITHER * diffuse;
//
// //let specular_exponent = clamp(1.0 - roughness, 0.1, 1.0) * 50.0;
// let specular_exponent = clamp(1.0 - roughness, 0.1, 1.0) * SPECULAR_INTENSITY;
// var specular = SPECULAR_MODIFIER * pow(saturate(dot(normal, half_dir)), specular_exponent);
// let specular_light = specular + dither * SPECULAR_DITHER * specular;
//
// let light = (ambient_light + diffuse_light + specular_light) * ao;
//
// let output = vec4<f32>(albedo * light, 1.0);
// //let output = vec4<f32>(albedo * light, 1.0);

// if btn4_pressed() {
//     return vec4<f32>(ambient_light, ambient_light, ambient_light, 1.0);
// }
// if btn5_pressed() {
//     return vec4<f32>(diffuse_light, diffuse_light, diffuse_light, 1.0);
// }
// if btn6_pressed() {
//     return vec4<f32>(specular_light, specular_light, specular_light, 1.0);
// }
// if btn4_pressed() {
//     return vec4<f32>(ao, ao, ao, 1.0);
// }
// if btn5_pressed() {
//     return vec4<f32>(roughness, roughness, roughness, 1.0);
// }
// if btn6_pressed() {
//     return vec4<f32>(metalness, metalness, metalness, 1.0);
// }
// if btn7_pressed() {
//     return vec4<f32>(ambient, ambient, ambient, 1.0);
// }
}

// fn rand(co: vec2<f32>) -> f32 {
//     return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
// }

const PI = 3.1415927;

//
// PBR
//

// directional lights
fn pbr_lighting(
    normal: vec3f,
    view_dir: vec3f,
    light_dir: vec3f,
    light_color: vec3f,
    albedo: vec3f,
    emissivity: vec3f,
    roughness: f32,
    metalness: f32,
    ambient_occlusion: f32,
) -> vec3f {
    let N = normalize(normal);
    let V = normalize(view_dir);
    let L = normalize(light_dir);
    let H = normalize(V + L);
    let F0 = mix(vec3f(0.04), albedo, metalness);

    let emission = emissivity;
    let radiance = light_color; // falloff when using point light
    let brdf = brdf_lambert_cook(roughness, metalness, F0, albedo, N, V, L, H);
    let ldotn = safe_dot(L, N);

    let light = emission + brdf * radiance * ldotn;

    let ambient = vec3f(0.03) * albedo * ambient_occlusion;
    let color = ambient + light;

    if btn8_pressed() {
        // specular (cook torrance)
        let F = fresnel_schlick(F0, V, H);
        let cook_torrance_num = distribution_trowbridge_ggx(roughness, N, H) * geometry_smith(roughness, N, V, L) * F;
        var cook_torrance_denom = 4.0 * safe_dot(V, N) * safe_dot(L, N);
        let cook_torrance = safe_division_vec3(cook_torrance_num, vec3f(cook_torrance_denom));
        return cook_torrance;
    }

    return color;
}

fn brdf_lambert_cook(
    roughness: f32,
    metalness: f32,
    F0: vec3f,
    albedo: vec3f,
    N: vec3f,
    V: vec3f,
    L: vec3f,
    H: vec3f,
) -> vec3f {
    // diffuse/specular distribution
    let F = fresnel_schlick(F0, V, H);
    let ks = F;
    let kd = (vec3f(1.0) - ks) * (1.0 - metalness);

    // diffuse (lambert)
    let lambert = albedo / PI;

    // specular (cook torrance)
    let cook_torrance_num = distribution_trowbridge_ggx(roughness, N, H) * geometry_smith(roughness, N, V, L) * F;
    var cook_torrance_denom = 4.0 * safe_dot(V, N) * safe_dot(L, N);
    let cook_torrance = safe_division_vec3(cook_torrance_num, vec3f(cook_torrance_denom));

    return kd * lambert + cook_torrance;
}

fn distribution_trowbridge_ggx(roughness: f32, N: vec3f, H: vec3f) -> f32 {
    let alpha = roughness;
    let alpha2 = alpha * alpha;
    let ndoth = safe_dot(N, H);
    let ndoth2 = ndoth * ndoth;

    let num = alpha2;
    let denom_part = (ndoth2 * (alpha2 - 1.0) + 1.0);
    let denom = PI * denom_part * denom_part;

    return safe_division_f32(num, denom);
}

fn geometry_schlick_ggx(roughness: f32, N: vec3f, X: vec3f) -> f32 {
    let k = roughness / 2.0;
    let ndotx = safe_dot(N, X);

    let num = ndotx;
    var denom = ndotx * (1.0 - k) + k;

    return safe_division_f32(num, denom);
}

fn geometry_smith(roughness: f32, N: vec3f, V: vec3f, L: vec3f) -> f32 {
    return geometry_schlick_ggx(roughness, N, V) * geometry_schlick_ggx(roughness, N, L);
}

fn fresnel_schlick(F0: vec3f, V: vec3f, H: vec3f) -> vec3f {
    return F0 + (vec3f(1.0) - F0) * pow(1 - safe_dot(V, H), 5.0);
}

// helpers

fn safe_dot(a: vec3f, b: vec3f) -> f32 {
    return max(dot(a, b), 0.0);
}

fn safe_division_f32(num: f32, denom: f32) -> f32 {
    return num / max(denom, 0.000001);
}

fn safe_division_vec3(num: vec3f, denom: vec3f) -> vec3f {
    return num / max(denom, vec3f(0.000001));
}
