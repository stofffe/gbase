struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var<uniform> app_info: AppInfo;
struct AppInfo {
    t: f32,
    screen_width: u32,
    screen_height: u32,
}

@group(0) @binding(1) var<uniform> camera: CameraUniform;
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

@group(0) @binding(2) var<uniform> bounding_box: Box3D;
struct Box3D {
    min: vec3f,
    max: vec3f,
}

@group(0) @binding(3) var<uniform> params: CloudParameters;
struct CloudParameters {
    light_pos: vec3f,

    alpha_cutoff: f32,
    henyey_forw: f32,
    henyey_back: f32,
    henyey_dist: f32,
}

@group(0) @binding(4) var noise_tex: texture_3d<f32>;
@group(0) @binding(5) var noise_samp: sampler;

@vertex
fn vs(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4f(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

const DENSITY_STEPS = 8;
const SUN_STEPS = 4;
const ABSORPTION = 13.0;
const ABSORPTION_DENSITY = ABSORPTION;
const ABSORPTION_SUN = ABSORPTION;
const TRANSMITTANCE_CUTOFF = 0.001;
const SUN_LIGHT_MULT = 5.0;
const CLOUD_SAMPLE_MULT = 0.25;
const SAMPLE_DENSITY_DISTRIBUTION = vec4f(3.0, 2.0, 1.0, 0.0);
const BEERS_MULT = 2.0;
const POWDER_MULT = 0.5;
const AMBIENT_LIGHT = 0.01;

//const ALPHA_CUTOFF = 0.9;
//const HENYEY_GREENSTEIN_FORW = 0.9; // how much scattering [0,1]
//const HENYEY_GREENSTEIN_BACK = 0.4; // how much scattering [0,1]
//const HENYEY_GREENSTEIN_DISTRIBUTION = 0.1; // forward [0,1] backwards

@fragment
fn fs(in: VertexOutput) -> @location(0) vec4f {
    let uv = in.uv;

    let ray = get_ray_dir(uv);

    let hit = ray_box_intersection(ray, bounding_box);
    let enter = max(hit.t_near, 0.0);
    let exit = hit.t_far;

    if !hit.hit {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    let hit_pos = ray.origin + ray.dir * enter;

    let cloud_info = cloud_march(ray, enter, exit);

    var alpha = 1.0 - cloud_info.transmittance;
    alpha = smoothstep(params.alpha_cutoff, 1.0, alpha);

    var color = cloud_info.color;
    color = max(color, vec3f(AMBIENT_LIGHT, AMBIENT_LIGHT, AMBIENT_LIGHT));

    return vec4f(color, alpha);
}

fn light_march(ray: Ray) -> vec3f {
    // assume hit
    let hit = ray_box_intersection(ray, bounding_box);
    let start = max(hit.t_near, 0.0);
    let end = min(hit.t_far, length(ray.origin - params.light_pos));
    let step_size = length(end - start) / f32(SUN_STEPS);

    var t = start;
    var transmittance = 1.0;

    for (var i = 0; i < SUN_STEPS; i++) {
        let pos = ray.origin + ray.dir * t;

        let density = (sample_density(pos));
        // let attenuation = beers(density, step_size, ABSORPTION_SUN); // how much ligth is absorbed un this step
        let attenuation = beers_powder(density, step_size, ABSORPTION_SUN); // how much ligth is absorbed un this step
        transmittance *= attenuation;

        if transmittance <= TRANSMITTANCE_CUTOFF {
            transmittance = 0.0;
            break;
        }

        t += step_size;
    }

    return vec3f(transmittance, transmittance, transmittance);
}

// returns transmittance
fn cloud_march(ray: Ray, entry: f32, exit: f32) -> CloudInfo {
    let step_size = length(exit - entry) / f32(DENSITY_STEPS);

    var t = entry;
    var transmittance = 1.0;
    var color = vec3f(0.0, 0.0, 0.0);

    for (var i = 0; i < DENSITY_STEPS; i++) {
        let pos = ray.origin + ray.dir * t;

        // opacity
        // let density = ease_in_cubic(sample_density(pos));
        let density = sample_density(pos);
        // let attenuation = beers(density, step_size, ABSORPTION); // how much ligth is absorbed un this step
        let attenuation = beers_powder(density, step_size, ABSORPTION); // how much ligth is absorbed un this step

        // color
        var light_ray: Ray;
        light_ray.origin = pos;
        light_ray.dir = normalize(params.light_pos - pos);
        var light = light_march(light_ray);
        light = light * dual_henyey_greenstein(dot(light_ray.dir, ray.dir), params.henyey_forw, params.henyey_back, params.henyey_dist);
        light *= SUN_LIGHT_MULT;
        light = saturate(light);
        color += light * transmittance * (1.0 - attenuation);

        transmittance *= attenuation;
        if transmittance <= TRANSMITTANCE_CUTOFF {
            transmittance = 0.0;
            break;
        }

        t += step_size;
    }

    var cloud_info: CloudInfo;
    cloud_info.transmittance = transmittance;
    cloud_info.color = color;
    return cloud_info;
}

fn beers(density: f32, distance: f32, absorption: f32) -> f32 {
    return exp(-(density * distance * absorption));
}

fn beers_powder(density: f32, distance: f32, absorption: f32) -> f32 {
    let powder = 1.0 - exp(-2.0 * density * distance * absorption);
    let beers = exp(-density * distance * absorption);
    return beers;
}

fn henyey_greenstein(g: f32, costheta: f32) -> f32 {
    return (1.0 / (4.0 * PI)) * ((1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * costheta, 1.5));
}

fn dual_henyey_greenstein(costheta: f32, g_forw: f32, g_back: f32, p: f32) -> f32 {
    return mix(henyey_greenstein(g_forw, costheta), henyey_greenstein(-g_back, costheta), p);
}

fn sample_density(pos: vec3f) -> f32 {
    let sampled_density = textureSample(noise_tex, noise_samp, pos * CLOUD_SAMPLE_MULT);
    let density = (SAMPLE_DENSITY_DISTRIBUTION.r * sampled_density.r
        + SAMPLE_DENSITY_DISTRIBUTION.g * sampled_density.g
        + SAMPLE_DENSITY_DISTRIBUTION.b * sampled_density.b
        + SAMPLE_DENSITY_DISTRIBUTION.a * sampled_density.a)
        / (SAMPLE_DENSITY_DISTRIBUTION.r + SAMPLE_DENSITY_DISTRIBUTION.g + SAMPLE_DENSITY_DISTRIBUTION.b + SAMPLE_DENSITY_DISTRIBUTION.a);

    return ease_in_cubic(density);
}

struct CloudInfo {
    transmittance: f32,
    color: vec3f,
}

struct Ray {
    origin: vec3f,
    dir: vec3f,
}

struct RayHit {
    hit: bool,
    t_near: f32,
    t_far: f32,

}

fn get_ray_dir(uv: vec2f) -> Ray {
    let ndc = vec4f(
        2.0 * uv.x - 1.0, // uv [0,1] -> ndc [-1,1]
        1.0 - 2.0 * uv.y, // flip y
        0.0, // can be anything
        1.0,
    );

    // revert view projection
    var world_pos = camera.inv_view_proj * ndc;
    world_pos /= world_pos.w; // homo -> world

    var ray: Ray;
    ray.dir = normalize(world_pos.xyz - camera.pos);
    ray.origin = camera.pos;

    return ray;
}

// Collisions

fn ray_box_intersection(ray: Ray, box: Box3D) -> RayHit {
    var t_min = vec3f(
        (box.min.x - ray.origin.x) / ray.dir.x,
        (box.min.y - ray.origin.y) / ray.dir.y,
        (box.min.z - ray.origin.z) / ray.dir.z,
    );

    var t_max = vec3f(
        (box.max.x - ray.origin.x) / ray.dir.x,
        (box.max.y - ray.origin.y) / ray.dir.y,
        (box.max.z - ray.origin.z) / ray.dir.z,
    );

    var t1 = min(t_min, t_max);
    var t2 = max(t_min, t_max);

    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);

    var result: RayHit;
    result.hit = t_far >= t_near && t_far >= 0.0;
    result.t_near = t_near;
    result.t_far = t_far;

    return result;
}

//
// Utils
//
const PI = 3.1415927;
const PI2 = PI * 2.0;
const PI1_2 = PI / 2.0;
const PI1_4 = PI / 4.0;
const PI1_8 = PI / 8.0;

// easeing functions
fn ease_out_cubic(x: f32) -> f32 {
    return 1.0 - pow(1.0 - x, 3.0);
}
fn ease_in_cubic(x: f32) -> f32 {
    return x * x * x;
}
fn ease_out_quad(x: f32) -> f32 {
    return 1.0 - (1.0 - x) * (1.0 - x);
}
fn ease_in_quad(x: f32) -> f32 {
    return x * x;
}
