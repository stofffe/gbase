@group(0) @binding(0) var<storage, read_write> instances: array<GrassInstance>;
@group(0) @binding(1) var<storage, read_write> instance_count: atomic<u32>;
@group(0) @binding(2) var<uniform> tile: Tile;
@group(0) @binding(3) var perlin_tex: texture_2d<f32>;
@group(0) @binding(4) var perlin_sam: sampler;
@group(0) @binding(5) var<uniform> camera: CameraUniform;


struct Tile {
    pos: vec2<f32>,
    size: f32,
    blades_per_side: f32,
};

// instances tightly packed => size must be multiple of align 
struct GrassInstance {          // align 16 size 48
    pos: vec3<f32>,             // align 16 size 12 start 0
    hash: u32,                  // align 4  size 4  start 12
    facing: vec2<f32>,          // align 8  size 8  start 16
    wind: vec2<f32>,            // align 8  size 8  start 24
    pad: vec3<f32>,             // align 16 size 12 start 32
    height: f32,                // align 4  size 4  start 44
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec3<f32>,
    facing: vec3<f32>,
};

@group(1) @binding(0) var<uniform> app_info: AppInfo;
struct AppInfo {
    time_passed: f32
};

@group(2) @binding(0) var<uniform> debug_input: DebugInput;
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

const ORTH_LIM = 0.4; // what dot_value orth rotation should start at
const ORTHOGONAL_ROTATE_MODIFIER = 1.0;
const ORTH_DIST_BOUNDS = vec2<f32>(2.0, 4.0); // between which distances to smoothstep orth rotation

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
    let hash = hash_2d(x, z);
    let blade_dist_between = tile.size / tile.blades_per_side;
    let blade_max_offset = blade_dist_between * 0.5;

    // POS
    let pos = vec3<f32>(
        tile.pos.x + f32(x) * blade_dist_between + hash_to_range_neg(hash) * blade_max_offset,
        0.0,
        (tile.pos.y + f32(z) * blade_dist_between + hash_to_range_neg(hash) * blade_max_offset),
    );

    // CULL
    var cull = false;
    if dot(camera.facing, pos - camera.pos) < 0.0 {
        cull = true;
    }


    if !cull {
        let t = app_info.time_passed;

        // FACING
        var facing = normalize(hash_to_vec2_neg(hash));
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

        // WIND
        // global wind from perline noise
        let tile_uv = vec2<f32>(f32(x), 1.0 - f32(z)) / tile.blades_per_side;
        let scroll = WIND_SCROLL_DIR * WIND_SCROLL_SPEED * t;
        let uv = tile_uv + scroll;
        // let global_wind_power = textureGather(1, perlin_tex, perlin_sam, uv).x; // think x = y = z // TODO filtering?
        let wind_sample_power = bilinear_r(uv);

        //let wind_sample_power = textureSample(perlin_tex, perlin_sam, uv) * WIND_GLOBAL_POWER;
        var global_wind_dir = normalize(WIND_DIR);
        var global_wind = vec2<f32>(
            abs(facing.x * global_wind_dir.x), // dot product on x 
            abs(facing.y * global_wind_dir.y), // dot product on z
        ) * global_wind_dir * wind_sample_power * WIND_GLOBAL_POWER;

        // blade curls towards normal, this affects how much wind is caught
        if global_wind.x * facing.x <= 0.0 {
            global_wind.x *= WIND_FACING_MODIFIER;
        }
        if global_wind.y * facing.y <= 0.0 {
            global_wind.y *= WIND_FACING_MODIFIER;
        }

        // local sway offset by hash
        let local_wind = vec2<f32>(
            facing.x * sin(t + 2.0 * PI * hash_to_range(hash)),
            facing.y * sin(t + 2.0 * PI * hash_to_range(hash ^ 0x732846u)),
        ) * WIND_LOCAL_POWER;

        let wind = global_wind + local_wind;

        // UPDATE INSTANCE DATA
        let i = atomicAdd(&instance_count, 1u);
        instances[i].pos = pos;
        instances[i].hash = hash;
        instances[i].facing = facing;
        instances[i].wind = wind;
        instances[i].height = 2.0;
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

// generates float in range [0, 1]
fn hash_to_range(hash: u32) -> f32 {
    return f32(hash) * 2.3283064e-10; // hash * 1 / 2^32
}

// generates float in range [-1, 1]
fn hash_to_range_neg(hash: u32) -> f32 {
    return (f32(hash) * 2.3283064e-10) * 2.0 - 1.0; // hash * 1 / 2^32
}

// generates vec2 with values in range [0, 1]
fn hash_to_vec2(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_range(hash ^ 0x36753621u),
        hash_to_range(hash ^ 0x12345678u),
    );
}

// generates vec2 with values in range [-1, 1]
fn hash_to_vec2_neg(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_range_neg(hash ^ 0x36753621u),
        hash_to_range_neg(hash ^ 0x12345678u),
    );
}

