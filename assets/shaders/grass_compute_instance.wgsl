@group(0) @binding(0) var<storage, read_write> instances: array<GrassInstance>;
@group(0) @binding(1) var<storage, read_write> instance_count: atomic<u32>;
@group(0) @binding(2) var<uniform> tile: Tile;
@group(0) @binding(3) var perlin_tex: texture_2d<f32>;
@group(0) @binding(4) var perlin_sam: sampler;
@group(0) @binding(5) var<uniform> camera: CameraUniform;
@group(0) @binding(6) var<uniform> app_info: AppInfo;
@group(0) @binding(7) var<uniform> debug_input: DebugInput;

struct Tile {
    pos: vec2<f32>,
    size: f32,
    blades_per_side: f32,
}


// instances tightly packed => size must be multiple of align
struct GrassInstance {
    pos: vec3<f32>,
    hash: u32,
    facing: vec2<f32>,
    wind: f32,
    pad: f32,
    height: f32,
    tilt: f32,
    bend: f32,
    width: f32,
}

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


struct AppInfo {
    time_passed: f32,
}


const ENABLE_INPUT = true;
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

fn btn1_pressed() -> bool { return debug_input.btn1 == 1u && ENABLE_INPUT; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u && ENABLE_INPUT; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u && ENABLE_INPUT; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u && ENABLE_INPUT; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u && ENABLE_INPUT; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u && ENABLE_INPUT; }
fn btn7_pressed() -> bool { return debug_input.btn7 == 1u && ENABLE_INPUT; }
fn btn8_pressed() -> bool { return debug_input.btn8 == 1u && ENABLE_INPUT; }
fn btn9_pressed() -> bool { return debug_input.btn9 == 1u && ENABLE_INPUT; }

// wind
const WIND_SCROLL_SPEED = 0.2;
const WIND_SCROLL_DIR = vec2<f32>(-1.0, 1.0); // TODO: something wrong with dir
const WIND_DIR = vec2<f32>(1.0, 1.0);
const WIND_MULTIPLIER = 0.5;
const WIND_TILT_MULTIPLIER = WIND_MULTIPLIER * 3.0;
const WIND_HEIGHT_MULTIPLIER = WIND_MULTIPLIER * 2.0;

// grass shape
const GRASS_OFFSET_MULTIPLIER = 0.5;

const GRASS_MIN_HEIGHT = 1.0;
const GRASS_MAX_HEIGHT = 2.5;

const GRASS_MIN_TILT = 0.0;
const GRASS_MAX_TILT = 1.0;

const GRASS_MIN_BEND = 0.1;
const GRASS_MAX_BEND = 0.5;

const GRASS_MIN_WIDTH = 0.1;
const GRASS_MAX_WIDTH = 0.15;

//const GRASS_MAX_ANGLE = 2.0 * PI;
const GRASS_MAX_ANGLE = PI / 4.0;

// culling
const GRASS_CULL_DIST = 100.0;
const GRASS_CULL_WIDTH_INCREASE = 3.0;

// clumps
const CLUMPS_PER_SIDE = 16;
const CLUMP_OFFSET_MULTIPLIER = 0.3;

// constants
const PI = 3.1415927;
const PI2 = PI * 2.0;
const PI1_2 = PI / 2.0;
const PI1_4 = PI / 4.0;
const PI1_8 = PI / 8.0;

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= u32(tile.blades_per_side) || global_id.y >= u32(tile.blades_per_side) {
        return;
    }

    let x = global_id.x;
    let z = global_id.y;
    let blade_hash = hash_2d(x, z);
    let blade_dist_between = tile.size / tile.blades_per_side;

    // POS
    var pos = vec3<f32>(
        tile.pos.x + f32(x) * blade_dist_between + hash_to_snorm(blade_hash) * blade_dist_between * GRASS_OFFSET_MULTIPLIER,
        0.0,
        tile.pos.y + f32(z) * blade_dist_between + hash_to_snorm(blade_hash ^ 0x2345624) * blade_dist_between * GRASS_OFFSET_MULTIPLIER,
    );

    // clumps
    let clump_dist_between = i32(tile.size) / CLUMPS_PER_SIDE;
    let center_clump_x = i32(f32(x) / tile.blades_per_side * tile.size / f32(clump_dist_between));
    let center_clump_z = i32(f32(z) / tile.blades_per_side * tile.size / f32(clump_dist_between));
    var closest_clump_pos = vec3<f32>(0.0);
    var closest_clump_dist = 999999.0;
    var closest_clump_hash = 0u;
    for (var j = -1; j <= 1; j += 1) {
        for (var i = -1; i <= 1; i += 1) {
            let cur_clump_x = center_clump_x + i;
            let cur_clump_z = center_clump_z + j;

            // oob check
            if cur_clump_x < 0 || cur_clump_z < 0 || cur_clump_x > CLUMPS_PER_SIDE || cur_clump_z > CLUMPS_PER_SIDE {
                continue;
            }

            let clump_hash = hash_2d(u32(cur_clump_x), u32(cur_clump_z)); // 28, ?
            let clump_pos = vec3<f32>(
                tile.pos.x + f32(cur_clump_x * clump_dist_between) + hash_to_snorm(clump_hash) * f32(clump_dist_between) * CLUMP_OFFSET_MULTIPLIER,
                0.0,
                tile.pos.y + f32(cur_clump_z * clump_dist_between) + hash_to_snorm(clump_hash ^ 0x42423432) * f32(clump_dist_between) * CLUMP_OFFSET_MULTIPLIER,
            );
            let clump_dist = length(pos - clump_pos);
            if clump_dist < closest_clump_dist {
                closest_clump_dist = clump_dist;
                closest_clump_pos = clump_pos;
                closest_clump_hash = clump_hash;
            }
        }
    }
    let clump_hash = closest_clump_hash;
    let clump_origin = closest_clump_pos;

    if btn3_pressed() {
        pos = mix(pos, clump_origin, 0.2);
    }
    if btn4_pressed() {
        pos = mix(pos, clump_origin, 1.0);
    }
    //pos = mix(pos, clump_origin, 0.3);

    // frustum cull
    // simple dot check, TODO imporve
    if dot(camera.facing, pos - camera.pos) < 0.0 {
        return;
    }
    // distance cull
    // cull 3/4 grass blades at distance
    if length(pos - camera.pos) > GRASS_CULL_DIST && (x % 2 == 0u || z % 2 == 0u) {
        return;
    }

    // facing angle
    //var facing = normalize(pos - clump_origin).xz;
    var facing_angle = 0.0;

    let blade_clump_dir = (pos - clump_origin).xz;

    facing_angle += atan2(blade_clump_dir.y, blade_clump_dir.x);
    facing_angle += hash_to_range(blade_hash, -GRASS_MAX_ANGLE, GRASS_MAX_ANGLE);
    let facing = normalize(vec2<f32>(cos(facing_angle), sin(facing_angle)));

    // caluclate wind
    let t = app_info.time_passed;
    let tile_uv = vec2<f32>(f32(x), 1.0 - f32(z)) / tile.blades_per_side;
    let scroll = WIND_SCROLL_DIR * WIND_SCROLL_SPEED * t;
    let wind_uv = tile_uv + scroll;
    let wind = bilinear_r(wind_uv);

    // adjust tilt based of wind
    let wind_facing_alignment = dot(facing, WIND_DIR);
    let wind_tilt = wind * wind_facing_alignment * WIND_TILT_MULTIPLIER;
    let tilt = hash_to_range(blade_hash, GRASS_MIN_TILT, GRASS_MAX_TILT) + wind_tilt;

    // adjust height beased of wind
    //let arclen = mix(GRASS_MIN_HEIGHT, GRASS_MAX_HEIGHT, bilinear_r(tile_uv * 5.0));
    let arclen = hash_to_range(blade_hash, GRASS_MIN_HEIGHT, GRASS_MAX_HEIGHT);
    let wind_height = -wind * saturate(wind_facing_alignment) * WIND_HEIGHT_MULTIPLIER; // TODO: acts weird when tilt becomes negative
    let height = arclen + wind_height;

    let bend = hash_to_range(blade_hash, GRASS_MIN_BEND, GRASS_MAX_BEND);
    var width = hash_to_range(blade_hash, GRASS_MIN_WIDTH, GRASS_MAX_WIDTH);
    let dist = length(pos - camera.pos);

    // TODO: enable again

    if dist > GRASS_CULL_DIST {
        width *= GRASS_CULL_WIDTH_INCREASE;
    }

    // update instance data
    let i = atomicAdd(&instance_count, 1u);
    instances[i].pos = pos;
    instances[i].hash = blade_hash;
    instances[i].facing = facing;
    instances[i].wind = wind;
    instances[i].height = height;
    instances[i].tilt = tilt;
    instances[i].bend = bend;
    instances[i].width = width;
}

//
// UTILS
//

fn bilinear_r(uv: vec2<f32>) -> f32 {
    let size = vec2<f32>(textureDimensions(perlin_tex));

    let tex = textureGather(0, perlin_tex, perlin_sam, uv);

    let offset = 1.0 / 512.0; // not needed?
    let weight = fract(uv * size - 0.5 + offset);
    //let weight = fract(uv * size - 0.5); // -0.5 since we have 4 pixels

    return mix(
        mix(tex.w, tex.z, weight.x),
        mix(tex.x, tex.y, weight.x),
        weight.y,
    );
}

// generates hash from two u32:s
fn hash_2d(x: u32, y: u32) -> u32 {
    // Use Wang hash for better distribution
    var hash: u32 = x;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
    return hash;
}

// TODO: temp
fn hash_2d_i32(x: i32, y: i32) -> i32 {
    // Use Wang hash for better distribution
    var hash: i32 = x;
    hash = hash ^ ((y << 16) | (y >> 16));
    hash = hash * 0x85ebca6;
    hash = hash ^ (hash >> 13);
    hash = hash * 0xc2b2ae3;
    hash = hash ^ (hash >> 16);
    return hash;
}

// generate float in range [low, high]
fn hash_to_range(hash: u32, low: f32, high: f32) -> f32 {
    return low + (high - low) * hash_to_unorm(hash);
}

// generates float in range [0, 1]
fn hash_to_unorm(hash: u32) -> f32 {
    return f32(hash) * 2.3283064e-10; // hash * 1 / 2^32
}

// generates float in range [-1, 1]
fn hash_to_snorm(hash: u32) -> f32 {
    return (f32(hash) * 2.3283064e-10) * 2.0 - 1.0; // hash * 1 / 2^32
}

// generates vec2 with values in range [0, 1]
fn hash_to_vec2_unorm(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_unorm(hash ^ 0x36753621u),
        hash_to_unorm(hash ^ 0x12345678u),
    );
}

// generates vec2 with values in range [-1, 1]
fn hash_to_vec2_snorm(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_snorm(hash ^ 0x36753621u),
        hash_to_snorm(hash ^ 0x12345678u),
    );
} // Rotate orthogonal verticies towards camera
