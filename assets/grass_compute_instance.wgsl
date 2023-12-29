@group(0) @binding(0) var<storage, read_write> instances: array<GrassInstance>;
@group(0) @binding(1) var<storage, read_write> instance_count: atomic<u32>;
@group(0) @binding(2) var<uniform> time_passed: f32;
// instances tightly packed => size must be multiple of align 
struct GrassInstance {          // align 16 size 32 
    pos: vec3<f32>,             // align 16 size 12 start 0
    hash: u32,                  // align 4  size 4  start 12
    facing: vec2<f32>,          // align 8  size 8  start 16
    wind: f32,                  // align 4  size 4  start 24
    pad: f32,                   // align 4  size 4  start 28
};

@group(1) @binding(0) var perlin_tex: texture_2d<f32>;
@group(1) @binding(1) var perlin_sam: sampler;

@group(2) @binding(0) var<uniform> camera: CameraUniform;
struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
};

const TILE_SIZE = 20.0;
const BLADES_PER_SIDE = 16.0 * 3.0;
const BLADE_DIST_BETWEEN = TILE_SIZE / BLADES_PER_SIDE;
const BLADE_MAX_OFFSET = BLADE_DIST_BETWEEN * 0.5;

const WIND_MODIFIER = 0.8;
const WIND_SCROLL_SPEED = 0.1;
const WIND_SCROLL_DIR = vec2<f32>(1.0, 1.0);

const ORTHOGONAL_LIMIT = 0.1;

const PI = 3.1415927;

@compute
@workgroup_size(16,16,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let z = global_id.y;
    let hash = hash_2d(x, z);

    // pos
    let pos = vec3<f32>(
        f32(x) * BLADE_DIST_BETWEEN + hash_to_range_neg(hash) * BLADE_MAX_OFFSET,
        0.0,
        f32(z) * BLADE_DIST_BETWEEN + hash_to_range_neg(hash) * BLADE_MAX_OFFSET,
    );

    // cull
    let cull = false;

    if !cull { 
        // wind power from perline noise
        let tile_pos = vec2<f32>(f32(x), 1.0 - f32(z)) / BLADES_PER_SIDE;
        let scroll = WIND_SCROLL_DIR * WIND_SCROLL_SPEED * time_passed ;
        let uv = tile_pos + scroll;
        let wind = textureGather(2, perlin_tex, perlin_sam, uv).x * WIND_MODIFIER; // think x = y = z

        // facing
        var facing = hash_to_vec2_neg(hash);

        let i = atomicAdd(&instance_count, 1u);
        instances[i].pos = pos;
        instances[i].hash = hash;
        instances[i].facing = facing;
        instances[i].wind = wind;
    }
}

// generates hash from two u32:s
fn hash_2d(x: u32, y: u32) -> u32 {
    var hash: u32 = x;
    hash = hash ^ (y << 16u);
    hash = (hash ^ (hash >> 16u)) * 0x45d9f3bu;
    hash = (hash ^ (hash >> 16u)) * 0x45d9f3bu;
    hash = hash ^ (hash >> 16u);
    return hash;
}

// generates vec2 with values in range [-1, 1]
fn hash_to_vec2_neg(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_range_neg(hash ^ 0x36753621u),
        hash_to_range_neg(hash ^ 0x12345678u),
    );
}

// generates float in range [0, 1]
fn hash_to_range(hash: u32) -> f32 {
    return f32(hash) * 2.3283064e-10; // hash * 1 / 2^32
}

// generates float in range [-1, 1]
fn hash_to_range_neg(hash: u32) -> f32 {
    return (f32(hash) * 2.3283064e-10) * 2.0 - 1.0; // hash * 1 / 2^32
}
