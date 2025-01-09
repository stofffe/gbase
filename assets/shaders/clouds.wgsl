struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

struct AppInfo {
    t: f32,
    screen_width: u32,
    screen_height: u32,
}

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

struct CloudParameters {
    light_pos: vec3f,
    bounds_min: vec3f,
    bounds_max: vec3f,

    alpha_cutoff: f32,
    density_cutoff: f32,
    henyey_forw: f32,
    henyey_back: f32,
    henyey_dist: f32,

    density_absorption: f32,
    sun_absorption: f32,
    transmittance_cutoff: f32,
    sun_light_mult: f32,
    cloud_sample_mult: f32,

    blue_noise_zoom: f32,
    blue_noise_step_mult: f32,

    sun_density_contribution_limit: f32,
}

@group(0) @binding(0) var<uniform> app_info: AppInfo;
@group(0) @binding(1) var<uniform> camera: CameraUniform;
@group(0) @binding(2) var<uniform> params: CloudParameters;
@group(0) @binding(3) var noise_tex: texture_3d<f32>;
@group(0) @binding(4) var weather_map_tex: texture_2d<f32>;
@group(0) @binding(5) var blue_noise_tex: texture_2d<f32>;
@group(0) @binding(6) var noise_samp: sampler;

@vertex
fn vs(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4f(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

const DENSITY_STEPS = 50;
const SUN_STEPS = 5;
// const DENSITY_STEPS = 10;
// const SUN_STEPS = 5;
const SUN_COLOR = vec3f(1.0, 1.0, 0.80);
const SCROLL_SPEED = 0.00;

@fragment
fn fs(in: VertexOutput) -> @location(0) vec4f {
    // if true {
    // let d = textureSample(noise_tex, noise_samp, vec3f(in.uv, 0.0)).b;
    // return vec4f(d, d, d, 1.0);
    // return textureSample(weather_map_tex, noise_samp, in.uv * vec2f(1.0, 1.0));
    // return textureSample(blue_noise_tex, noise_samp, in.uv * 2.0).a * vec4f(1.0);
    // }

    let uv = in.uv;

    let ray = get_ray_dir(uv);

    var bounding_box: Box3D;
    bounding_box.min = params.bounds_min;
    bounding_box.max = params.bounds_max;

    let hit = ray_box_intersection(ray, bounding_box);
    var entry = max(hit.t_near, 0.0);
    let exit = hit.t_far;

    // offset entry with blue noise
    let blue_noise = textureSample(blue_noise_tex, noise_samp, uv * params.blue_noise_zoom).r;
    let step_size = length(exit - entry) / f32(DENSITY_STEPS);
    let entry_offseted = entry + (blue_noise - 0.5) * params.blue_noise_step_mult * step_size;

    if !hit.hit {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    let hit_pos = ray.origin + ray.dir * entry;

    let cloud_info = cloud_march(ray, entry_offseted, exit, blue_noise);

    var alpha = 1.0 - cloud_info.transmittance;
    alpha = smoothstep(params.alpha_cutoff, 1.0, alpha);

    var color = cloud_info.color;

    return vec4f(color, alpha);
}

// get attenuation to sun
fn light_march(main_ray: Ray, sun_ray: Ray, blue_noise: f32) -> vec3f {
    // NOTE: assume hit
    var bounding_box: Box3D;
    bounding_box.min = params.bounds_min;
    bounding_box.max = params.bounds_max;

    let hit = ray_box_intersection(sun_ray, bounding_box);
    let start = max(hit.t_near, 0.0);
    let end = min(hit.t_far, length(sun_ray.origin - params.light_pos));
    let dist = (end - start);
    let step_size = dist / f32(SUN_STEPS);

    var t = start;
    var density = 0.0;
    for (var i = 0; i < SUN_STEPS; i++) {
        let pos = sun_ray.origin + sun_ray.dir * t;

        density += sample_density(pos) / f32(SUN_STEPS);
        if density >= 1.0 {
            break;
        }

        t += step_size;
    }

    let costh = dot(sun_ray.dir, main_ray.dir);

    var attenuation = beers(density, end - start, params.sun_absorption) * multiple_octave_scattering(density, costh);
    // attenuation += blue_noise * 0.005;

    // NOTE: not used
    let powder = 1.0 - exp(-2.0 * density * params.sun_absorption);

    return attenuation * params.sun_light_mult * SUN_COLOR + powder * 2.0;
// return attenuation * params.sun_light_mult * SUN_COLOR * remap(powder, 0.0, 1.0, 0.6, 1.0);
}

// returns transmittance
fn cloud_march(ray: Ray, entry: f32, exit: f32, blue_noise: f32) -> CloudInfo {
    let steps = f32(DENSITY_STEPS);
    let step_size = length(exit - entry) / steps;

    var t = entry;
    var transmittance = 1.0;
    var color = vec3f(0.0, 0.0, 0.0);

    for (var i = 0; i < DENSITY_STEPS; i++) {
        let pos = ray.origin + ray.dir * t;

        // opacity
        let density = sample_density(pos);
        let attenuation = beers(density, step_size, params.density_absorption); // how much ligth is absorbed un this step

        // color
        var light_ray: Ray;
        light_ray.origin = pos;
        light_ray.dir = normalize(params.light_pos - pos);

        // only calc light if density large enough
        if density >= params.sun_density_contribution_limit {
            let light = light_march(ray, light_ray, blue_noise);
            // NOTE:
            // 1.0 - attenuation: amount that is being absorbed in this point
            // transmittance: amount of light in this spot that reaches camera
            color += light * (1.0 - attenuation) * transmittance;

        }

        transmittance *= attenuation;
        if transmittance <= params.transmittance_cutoff {
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


fn sample_density(pos: vec3f) -> f32 {
    // noise
    let noise_coord_offset = vec3f(1.0, 0.0, 1.0) * app_info.t * SCROLL_SPEED;
    let noise_coords = pos / params.cloud_sample_mult + noise_coord_offset;
    let noise = textureSample(noise_tex, noise_samp, noise_coords);
    let perlin = noise.a;
    var worley = (0.625 * noise.r + 0.25 * noise.g + 0.125 * noise.b);
    worley = remap(worley, params.density_cutoff, 1.0, 0.0, 1.0);
    let carve = saturate(remap(perlin, worley, 1.0, 0.0, 1.0));

    // weather
    let weather_coord_offset = vec2f(1.0, 1.0) * app_info.t * SCROLL_SPEED;
    let weather_coord = pos.xz * vec2f(1.0 / 1.0, 1.0) * 0.003 + weather_coord_offset;
    let weather = textureSample(weather_map_tex, noise_samp, weather_coord);
    let weather_mask = weather.r;
    let weather_height = weather.g + 0.1;

    // top/bottom rounding
    let ph = (pos.y - params.bounds_min.y) / (params.bounds_max.y - params.bounds_min.y);
    let bottom_rounding = saturate(remap(ph, 0.0, 0.07, 0.0, 1.0)); // round bottom
    let top_rounding = saturate(remap(ph, 0.1 * weather_height, weather_height, 1.0, 0.0)); // round top

    return weather_mask * bottom_rounding * top_rounding * carve;
}

// calculate attenuation with beers law
fn beers(density: f32, distance: f32, absorption: f32) -> f32 {
    return exp(-(density * distance * absorption));
}

// calculate attenuation with beers law and powder equation
fn beers_powder(density: f32, distance: f32, absorption: f32) -> f32 {
    let powder = 1.0 - exp(-2.0 * density * distance * absorption);
    let beers = exp(-density * distance * absorption);
    return beers * powder;
}

// calculate forward scattering
fn henyey_greenstein(g: f32, costheta: f32) -> f32 {
    return (1.0 / (4.0 * PI)) * ((1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * costheta, 1.5));
}

// calculate forward/back scattering
fn dual_henyey_greenstein(costheta: f32, g_forw: f32, g_back: f32, p: f32) -> f32 {
    return mix(henyey_greenstein(g_forw, costheta), henyey_greenstein(-g_back, costheta), p);
}

fn multiple_octave_scattering(density: f32, costh: f32) -> vec3f {
    const attenuation = 0.2;
    const contribution = 0.2;
    const phaseAttenuation = 0.5;
    const scatteringOctaves = 4;

    var a = 1.0;
    var b = 1.0;
    var c = 1.0;
    var luminance = vec3f(0.0);
    for (var i = 0; i < scatteringOctaves; i += 1) {
        let phaseFunction = dual_henyey_greenstein(costh, params.henyey_forw, params.henyey_back, params.henyey_dist);
        let beer = exp(-density * params.sun_absorption * a);

        luminance = luminance + b * phaseFunction * beer;

        a = a * attenuation;
        b = b * contribution;
        c = c * (1.0 - phaseAttenuation);
    }

    return luminance;
}

struct CloudInfo {
    transmittance: f32,
    color: vec3f,
}

struct Box3D {
    min: vec3f,
    max: vec3f,
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

fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    return to_min + (to_max - to_min) * ((value - from_min) / (from_max - from_min));
}

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
