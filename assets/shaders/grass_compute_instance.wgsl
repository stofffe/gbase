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
};

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
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
    facing: vec3<f32>,
};

struct AppInfo {
    time_passed: f32
};

struct DebugInput { btn1: u32, btn2: u32, btn3: u32, btn4: u32, btn5: u32, btn6: u32, btn7: u32, btn8: u32, btn9: u32 };
fn btn1_pressed() -> bool { return debug_input.btn1 == 1u; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u; }

//const WIND_GLOBAL_POWER = 2.0;
//const WIND_LOCAL_POWER = 0.05;
const WIND_GLOBAL_POWER = 2.0;
const WIND_LOCAL_POWER = 0.1;
const WIND_SCROLL_SPEED = 0.1;
const WIND_SCROLL_DIR = vec2<f32>(1.0, 1.0);
const WIND_DIR = vec2<f32>(1.0, 1.0); // TODO sample from texture instead
const WIND_FACING_MODIFIER = 2.0;

const GRASS_MIN_HEIGHT = 1.0;
const GRASS_MAX_HEIGHT = 4.0;

const GRASS_MIN_TILT = 0.0;
const GRASS_MAX_TILT = 1.0;

const GRASS_MIN_BEND = 0.1;
const GRASS_MAX_BEND = 0.2;

const GRASS_MIN_WIDTH = 0.1;
const GRASS_MAX_WIDTH = 0.15;

const ORTH_LIM = 0.4; // what dot_value orth rotation should start at
const ORTHOGONAL_ROTATE_MODIFIER = 1.0;
const ORTH_DIST_BOUNDS = vec2<f32>(2.0, 4.0); // between which distances to smoothstep orth rotation

const GRASS_CULL_DIST = 100.0;

const PI = 3.1415927;

@compute
@workgroup_size(16,16,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // debug
    //if global_id.x >= 1u || global_id.y >= 1u {
    //    return;
    //}
    if global_id.x >= u32(tile.blades_per_side) || global_id.y >= u32(tile.blades_per_side) {
        return;
    }

    let x = global_id.x;
    let z = global_id.y;
    //let hash = hash_2d(x, z);
    let hash = hash_2d_improved(x, z);
    let blade_dist_between = tile.size / tile.blades_per_side;
    let blade_max_offset = blade_dist_between * 0.5;

    // POS
    let pos = vec3<f32>(
        tile.pos.x + f32(x) * blade_dist_between + hash_to_snorm(hash) * blade_max_offset,
        0.0,
        (tile.pos.y + f32(z) * blade_dist_between + hash_to_snorm(hash) * blade_max_offset),
    );

    // CULL
    var cull = false;
    if dot(camera.facing, pos - camera.pos) < 0.0 {
        cull = true;
    }

    // cull 3/4 grass blades at distance

    let dist = length(pos - camera.pos);
    //if dist > 100.0 && (x % 2 == 0u) {
    //    cull = true;
    //}
    if dist > GRASS_CULL_DIST && (x % 2 == 0u || z % 2 == 0u) {
        cull = true;
    }
    //let dist = length(camera.pos - pos);
    //if hash_to_range(hash) > 500.0 / pow(length(camera.pos - pos), 2.0) {
    //    cull = true;
    //}


    if !cull {
        let t = app_info.time_passed;

        // FACING
        let facing_angle = hash_to_range(hash, 0.0, 2.0 * PI);
        var facing = normalize(vec2<f32>(
            cos(facing_angle),
            sin(facing_angle)
        ));
        // Rotate orthogonal verticies towards camera 
        //let camera_dir = normalize(camera.pos.xz - pos.xz);
        //let dist_modifier = smoothstep(ORTH_DIST_BOUNDS.x, ORTH_DIST_BOUNDS.y, length(camera.pos.xz - pos.xz));
        //let vnd = dot(camera_dir, facing); // view normal dot
        //if vnd >= 0.0 {
        //    let rotate_factor = pow(1.0 - vnd, 3.0) * smoothstep(0.0, ORTH_LIM, vnd) * ORTHOGONAL_ROTATE_MODIFIER * dist_modifier;
        //    facing = mix(facing, camera_dir, rotate_factor);
        //} else {
        //    let rotate_factor = pow(vnd + 1.0, 3.0) * smoothstep(ORTH_LIM, 0.0, vnd + ORTH_LIM) * ORTHOGONAL_ROTATE_MODIFIER * dist_modifier;
        //    facing = mix(facing, -camera_dir, rotate_factor);
        //}

        // let global_wind_power = textureGather(1, perlin_tex, perlin_sam, uv).x; // think x = y = z // TODO filtering?

        // WIND
        let tile_uv = vec2<f32>(f32(x), 1.0 - f32(z)) / tile.blades_per_side;
        let scroll = WIND_SCROLL_DIR * WIND_SCROLL_SPEED * t;
        let wind_uv = tile_uv + scroll;
        let wind = bilinear_r(wind_uv);
        // global wind from perline noise
        //let tile_uv = vec2<f32>(f32(x), 1.0 - f32(z)) / tile.blades_per_side;
        //let scroll = WIND_SCROLL_DIR * WIND_SCROLL_SPEED * t;
        //let uv = tile_uv + scroll;
        //let wind_sample_power = bilinear_r(uv);

        //let wind_sample_power = textureSample(perlin_tex, perlin_sam, uv) * WIND_GLOBAL_POWER;
        //var global_wind_dir = normalize(WIND_DIR);
        //var global_wind = vec2<f32>(
        //    abs(facing.x * global_wind_dir.x), // dot product on x 
        //    abs(facing.y * global_wind_dir.y), // dot product on z
        //) * global_wind_dir * wind_sample_power * WIND_GLOBAL_POWER;

        // blade curls towards normal, this affects how much wind is caught
        //if global_wind.x * facing.x <= 0.0 {
        //    global_wind.x *= WIND_FACING_MODIFIER;
        //}
        //if global_wind.y * facing.y <= 0.0 {
        //    global_wind.y *= WIND_FACING_MODIFIER;
        //}

        // local sway offset by hash
        //let local_wind = vec2<f32>(
        //    facing.x * sin(t + 2.0 * PI * hash_to_unorm(hash)),
        //    facing.y * sin(t + 2.0 * PI * hash_to_unorm(hash ^ 0x732846u)),
        //) * WIND_LOCAL_POWER;

        //let wind = global_wind + local_wind;

        // UPDATE INSTANCE DATA
        let i = atomicAdd(&instance_count, 1u);
        instances[i].pos = pos;
        instances[i].hash = hash;
        instances[i].facing = facing;
        instances[i].wind = wind;
        instances[i].height = mix(GRASS_MIN_HEIGHT, GRASS_MAX_HEIGHT, bilinear_r(tile_uv * 5.0));
        // instances[i].height = mix(GRASS_MIN_HEIGHT, GRASS_MAX_HEIGHT, bilinear_r(tile_uv * 5.0)) + hash_to_range(hash, -0.5, 0.5);
        instances[i].tilt = hash_to_range(hash, GRASS_MIN_TILT, GRASS_MAX_TILT);
        instances[i].bend = hash_to_range(hash, GRASS_MIN_BEND, GRASS_MAX_BEND);
        instances[i].width = hash_to_range(hash, GRASS_MIN_WIDTH, GRASS_MAX_WIDTH);
        if dist > GRASS_CULL_DIST {
            instances[i].width *= 3.0;
        }
    }
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
    var hash: u32 = x;
    hash = hash ^ (y << 16u);
    hash = (hash ^ (hash >> 16u)) * 0x45d9f3bu;
    hash = (hash ^ (hash >> 16u)) * 0x45d9f3bu;
    hash = hash ^ (hash >> 16u);
    return hash;
}

// generates hash from two u32:s
fn hash_2d_improved(x: u32, y: u32) -> u32 {
    // Use Wang hash for better distribution
    var hash: u32 = x;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
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
}
