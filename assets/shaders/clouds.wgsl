struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> app_info: AppInfo;
struct AppInfo {
    t: f32,
    screen_width: u32,
    screen_height: u32,
}


@group(0) @binding(1) var<uniform> camera: CameraUniform;
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

@group(0) @binding(2) var<uniform> bounding_box: Box3D;
struct Box3D {
    min: vec3<f32>,
    max: vec3<f32>,
}

@group(0) @binding(3) var noise_tex: texture_3d<f32>;
@group(0) @binding(4) var noise_samp: sampler;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    let ray = get_ray_dir(uv);

    let hit = ray_box_intersection(ray, bounding_box);
    let enter = max(hit.t_near, 0.0);
    let exit = hit.t_far;

    var color: vec3f;
    if hit.hit {
        let d = exit - enter;
        color = vec3f(d, d, d);
    } else {
        color = vec3f(0.0, 0.0, 0.0);
    }

    return vec4<f32>(color, 1.0);
}

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
}

struct RayHit {
    hit: bool,
    t_near: f32,
    t_far: f32,

}

fn get_ray_dir(uv: vec2<f32>) -> Ray {

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
    var t_min = vec3<f32>(
        (box.min.x - ray.origin.x) / ray.dir.x,
        (box.min.y - ray.origin.y) / ray.dir.y,
        (box.min.z - ray.origin.z) / ray.dir.z,
    );

    var t_max = vec3<f32>(
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

// constants
const PI = 3.1415927;
const PI2 = PI * 2.0;
const PI1_2 = PI / 2.0;
const PI1_4 = PI / 4.0;
const PI1_8 = PI / 8.0; //fn get_ray_dir_2(uv: vec2<f32>) -> vec3<f32> {
