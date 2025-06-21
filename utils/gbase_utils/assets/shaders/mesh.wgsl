struct CameraUniform {
    pos: vec3<f32>,
    facing: vec3<f32>,

    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,

    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}

struct PbrMaterial {
    base_color_factor: vec4f,
    roughness_factor: f32,
    metallic_factor: f32,
    occlusion_strength: f32,
    normal_scale: f32,
}

struct PbrLights {
    main_light_dir: vec3f,
    main_light_intensity: f32,
}
struct VertexInput {
    @builtin(instance_index) index: u32,
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tangent: vec4f,
    @location(3) uv: vec2f,
    @location(4) color: vec3f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> lights: PbrLights;
@group(0) @binding(2) var<storage, read> instances: array<Instance>;
@group(0) @binding(3) var base_color_texture: texture_2d<f32>;
@group(0) @binding(4) var base_color_sampler: sampler;
@group(0) @binding(5) var normal_texture: texture_2d<f32>;
@group(0) @binding(6) var normal_sampler: sampler;
@group(0) @binding(7) var metallic_roughness_texture: texture_2d<f32>;
@group(0) @binding(8) var metallic_roughness_sampler: sampler;
@group(0) @binding(9) var occlusion_texture: texture_2d<f32>;
@group(0) @binding(10) var occlusion_sampler: sampler;
@group(0) @binding(11) var emissive_texture: texture_2d<f32>;
@group(0) @binding(12) var emissive_sampler: sampler;
@group(0) @binding(13) var shadow_map_texture: texture_depth_2d;
@group(0) @binding(14) var shadow_map_sampler: sampler;
@group(0) @binding(15) var<uniform> shadow_matrix: mat4x4f;

// NOTE: alignment
struct Instance {
    // transform
    transform: mat4x4f,

    // material
    color_factor: vec4f,
    roughness_factor: f32,
    metallic_factor: f32,
    occlusion_strength: f32,
    normal_scale: f32,
    emissive_factor: vec3f,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let model = instances[in.index].transform;
    // NOTE: w component of tangent shoudl specify LH RH coordinate system
    // always assume RH so ignore this value
    let T = normalize((model * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    let N = normalize((model * vec4<f32>(in.normal, 0.0)).xyz);
    let B = cross(N, T);

    var out: VertexOutput;
    let position = model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * position;
    out.uv = in.uv;
    out.color = in.color;
    // NOTE: since TBN rotates normal and no normal texture is used assume normal is (0,0,1)
    // need to move this step to fragment shader if using normal textures
    out.pos = position.xyz;
    out.T = T;
    out.B = B;
    out.N = N;
    out.index = in.index;
    out.light_pos = shadow_matrix * position;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3f,
    @location(1) color: vec3f,
    @location(2) uv: vec2f,
    @location(5) T: vec3f,
    @location(6) B: vec3f,
    @location(7) N: vec3f,
    @location(8) index: u32,
    @location(9) light_pos: vec4f,
}

const BIAS = 0.005;
const MIN_BIAS = 0.005;
const MAX_BIAS = 0.010;
fn in_shadow(light_pos: vec4f, normal: vec3f, light_dir: vec3f) -> bool {
    var proj_coords = light_pos / light_pos.w;
    proj_coords.x = proj_coords.x * 0.5 + 0.5;
    proj_coords.y = proj_coords.y * 0.5 + 0.5;
    proj_coords.y = 1.0 - proj_coords.y;
    let shadow_map_depth = textureSample(shadow_map_texture, shadow_map_sampler, proj_coords.xy);
    let pixel_depth = proj_coords.z;

    // check bounds
    if any(proj_coords.xy < vec2f(0.0)) || any(proj_coords.xy > vec2f(1.0)) {
        return false;
    }

    var bias = max(MAX_BIAS * (1.0 - dot(normal, light_dir)), MIN_BIAS);
    // bias = BIAS;

    // return false;
    return saturate(pixel_depth) > shadow_map_depth + bias;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    if false {
        var proj_coords = in.light_pos / in.light_pos.w;
        proj_coords.x = proj_coords.x * 0.5 + 0.5;
        proj_coords.y = proj_coords.y * 0.5 + 0.5;
        proj_coords.y = 1.0 - proj_coords.y;
        let shadow_map_depth = textureSample(shadow_map_texture, shadow_map_sampler, proj_coords.xy);
        let pixel_depth = proj_coords.z;

        // check bounds
        if any(proj_coords.xy < vec2f(0.0)) || any(proj_coords.xy > vec2f(1.0)) {
            return vec4f(0.0, 0.0, 1.0, 1.0);
        }

        if pixel_depth < 0.0 {
            return vec4f(1.0, 0.0, 0.0, 1.0);
        }

        if pixel_depth > 1.0 {
            return vec4f(0.0, 1.0, 0.0, 1.0);
        }

    }

    let instance = instances[in.index];

    let base_color_tex = decode_gamma_correction(textureSample(base_color_texture, base_color_sampler, in.uv));
    let normal_tex = textureSample(normal_texture, normal_sampler, in.uv);
    let roughness_tex = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
    let occlusion_tex = textureSample(occlusion_texture, occlusion_sampler, in.uv);
    let emissive_tex = decode_gamma_correction(textureSample(emissive_texture, emissive_sampler, in.uv));

    let albedo = base_color_tex.xyz * in.color * instance.color_factor.xyz;
    let emissive = emissive_tex.xyz * instance.emissive_factor;

    let roughness = roughness_tex.g * instance.roughness_factor;
    let metalness = roughness_tex.b * instance.metallic_factor;
    let occlusion = 1.0 + instance.occlusion_strength * (occlusion_tex.r - 1.0);

    let TBN = mat3x3f(in.T, in.B, in.N);
    let unpacked_normal = normalize(normal_tex.xyz * 2.0 - 1.0) * instance.normal_scale; // [0,1] -> [-1,1]
    let normal = normalize(TBN * unpacked_normal);

    let light_dir = -lights.main_light_dir;
    let view_dir = camera.pos - in.pos;
    let light_color = vec3f(1.0) * lights.main_light_intensity;

    if in_shadow(in.light_pos, normal, light_dir) {
        return base_color_tex * 0.01;
    }

    // main light
    let color = pbr_lighting(
        normal,
        view_dir,
        light_dir,
        light_color,
        albedo,
        emissive,
        roughness,
        metalness,
        occlusion,
    );

    if true {
    // return vec4f(albedo, 1.0);
    // return vec4f(instance.color_factor);
    // return vec4f(roughness, roughness, roughness, 1.0);
    // return vec4f(metalness, mtalness, metalness, 1.0);
    }
    return vec4f(color, 1.0);

}

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
    let radiance = light_color; // TODO: falloff when using point light
    let brdf = brdf_lambert_cook(roughness, metalness, F0, albedo, N, V, L, H);
    let ldotn = safe_dot(L, N);

    let light = emission + brdf * radiance * ldotn;

    let ambient = vec3f(0.03) * albedo * ambient_occlusion;
    let hdr_color = ambient + light;

    // let ldr_color = tone_mapping_reinhard(hdr_color);
    let ldr_color = hdr_color;

    return ldr_color;
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

fn decode_gamma_correction(in: vec4f) -> vec4f {
    return pow(in, vec4f(2.2));
}

fn encode_gamma_correction(in: vec4f) -> vec4f {
    return pow(in, vec4f(1.0 / 2.2));
}

fn tone_mapping_reinhard(color: vec3f) -> vec3f {
    return color / (color + vec3f(1.0));
}
