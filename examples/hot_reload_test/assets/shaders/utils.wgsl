//
// Remap values from one range to another
//

fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    return to_min + (to_max - to_min) * ((value - from_min) / (from_max - from_min));
}

//
// Modulo with negative support
//

fn mod_f(value: f32, n: f32) -> f32 { return (value + n) % n; }
fn mod_vec2f_f(value: vec2<f32>, n: f32) -> vec2<f32> { return (value + n) % n; }
fn mod_vec3f_f(value: vec3<f32>, n: f32) -> vec3<f32> { return (value + n) % n; }
fn mod_vec4f_f(value: vec4<f32>, n: f32) -> vec4<f32> { return (value + n) % n; }
fn mod_vec3f_vec3f(value: vec3<f32>, n: vec3f) -> vec3<f32> { return (value + n) % n; }

fn mod_i32(value: i32, n: i32) -> i32 { return (value + n) % n; }
fn mod_vec2_i32(value: vec2<i32>, n: i32) -> vec2<i32> { return (value + n) % n; }
fn mod_vec3_i32(value: vec3<i32>, n: i32) -> vec3<i32> { return (value + n) % n; }
fn mod_vec4_i32(value: vec4<i32>, n: i32) -> vec4<i32> { return (value + n) % n; }

fn mod_u32(value: u32, n: u32) -> u32 { return (value + n) % n; }
fn mod_vec2_u32(value: vec2<u32>, n: u32) -> vec2<u32> { return (value + n) % n; }
fn mod_vec3_u32(value: vec3<u32>, n: u32) -> vec3<u32> { return (value + n) % n; }
fn mod_vec4_u32(value: vec4<u32>, n: u32) -> vec4<u32> { return (value + n) % n; }

//
// Tiling 3d perlin fbm
//

fn hash3d(pos: vec3f) -> f32 { return fract(sin(dot(pos, vec3<f32>(27.16898, 38.90563, 43.23476))) * 5151.5473453); }

fn perlin_3d(pos: vec3f, scale: f32) -> f32 {
    var f: vec3f;
    var p = pos * scale;
    f = fract(p);
    p = floor(p);
    f = f * f * (3.0 - 2.0 * f);

    let c000 = hash3d(mod_vec3f_f(p, scale));
    let c100 = hash3d(mod_vec3f_f(p + vec3<f32>(1.0, 0.0, 0.0), scale));
    let c010 = hash3d(mod_vec3f_f(p + vec3<f32>(0.0, 1.0, 0.0), scale));
    let c110 = hash3d(mod_vec3f_f(p + vec3<f32>(1.0, 1.0, 0.0), scale));
    let c001 = hash3d(mod_vec3f_f(p + vec3<f32>(0.0, 0.0, 1.0), scale));
    let c101 = hash3d(mod_vec3f_f(p + vec3<f32>(1.0, 0.0, 1.0), scale));
    let c011 = hash3d(mod_vec3f_f(p + vec3<f32>(0.0, 1.0, 1.0), scale));
    let c111 = hash3d(mod_vec3f_f(p + vec3<f32>(1.0, 1.0, 1.0), scale));

    return mix(mix(mix(c000, c100, f.x), mix(c010, c110, f.x), f.y), mix(mix(c001, c101, f.x), mix(c011, c111, f.x), f.y), f.z);
}

fn perlin_fbm_3d(p: vec3f, noise_scale: f32) -> f32 {
    var f = 0.0;

    var scale = noise_scale;
    var amp = 0.5;
    for (var i = 0; i < 5; i = i + 1) {
        f += perlin_3d(p, scale) * amp;
        amp = amp * 0.5;
        scale = scale * 2.0;
    }

    return saturate(f);
}

//
// Tiling 2d perlin fbm
//

fn hash2d(pos: vec2f) -> f32 {
    return fract(sin(dot(pos, vec2<f32>(27.16898, 38.90563))) * 5151.5473453);
}
fn perlin_2d(pos: vec2f, scale: f32) -> f32 {
    var f: vec2f;
    var p = pos * scale;
    f = fract(p);
    p = floor(p);
    f = f * f * (3.0 - 2.0 * f);

    let c00 = hash2d(mod_vec2f_f(p, scale));
    let c10 = hash2d(mod_vec2f_f(p + vec2<f32>(1.0, 0.0), scale));
    let c01 = hash2d(mod_vec2f_f(p + vec2<f32>(0.0, 1.0), scale));
    let c11 = hash2d(mod_vec2f_f(p + vec2<f32>(1.0, 1.0), scale));

    return mix(mix(c00, c10, f.x), mix(c01, c11, f.x), f.y);
}
fn perlin_fbm_2d(p: vec2<f32>) -> f32 {
    var f: f32 = 0.0;
    var scale: f32 = 10.0;
    var amp: f32 = 0.6;

    for (var i = 0; i < 5; i = i + 1) {
        f = f + perlin_2d(p, scale) * amp;
        amp = amp * 0.5;
        scale = scale * 2.0;
    }

    return min(f, 1.0);
}

//
//
//

//
// Frustum culling
//
struct Plane {
    origin: vec3f,
    normal: vec3f,
}
struct CameraFrustum {
    near: Plane,
    far: Plane,
    left: Plane,
    right: Plane,
    bottom: Plane,
    top: Plane,
}
fn frustum_sphere_inside(
    frustum: CameraFrustum,
    origin: vec3f,
    radius: f32,
) -> bool {
    let inside_near = dot(origin - frustum.near.origin, frustum.near.normal) + radius >= 0.0;
    let inside_far = dot(origin - frustum.far.origin, frustum.far.normal) + radius >= 0.0;
    let inside_left = dot(origin - frustum.left.origin, frustum.left.normal) + radius >= 0.0;
    let inside_right = dot(origin - frustum.right.origin, frustum.right.normal) + radius >= 0.0;
    let inside_bottom = dot(origin - frustum.bottom.origin, frustum.bottom.normal) + radius >= 0.0;
    let inside_top = dot(origin - frustum.top.origin, frustum.top.normal) + radius >= 0.0;
    return inside_near && inside_far
        && inside_left && inside_right
        && inside_bottom && inside_top;
}

//
// Lighting
//

fn calculate_luminance(color: vec3f) -> f32 {
    return dot(color, vec3f(0.2126, 0.7152, 0.0722));
}

//
// Depth
//

// Convert depth value to the actual depth value in range [near, far]
fn linearize_depth(depth: f32, near: f32, far: f32) -> f32 {
    return (2.0 * near * far) / (far + near - depth * (far - near));
}

// Convert depth value to the actual depth value in range [0, 1]
fn linearize_depth_normalized(depth: f32, near: f32, far: f32) -> f32 {
    return (2.0 * near * far) / (far + near - depth * (far - near)) / far;
}
