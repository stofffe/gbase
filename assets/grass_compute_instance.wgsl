@group(0) @binding(0) var<storage, read_write> instances: array<GrassInstance>;
@group(0) @binding(1) var<storage, read_write> instance_count: atomic<u32>;

// instances tightly packed => size must be multiple of align 
struct GrassInstance {          // align 16 size 32 
    pos: vec3<f32>,             // align 16 size 12 start 0
    hash: u32,                  // align 4  size 4  start 12
    facing: vec2<f32>,          // align 8  size 8  start 16
    wind: f32,                  // align 4  size 4  start 24
    pad: f32,                   // align 4  size 4  start 28
};

const blades_per_side = 16.0 * 1.0;
const tile_size = 5.0;

const PI = 3.1415927;

@compute
@workgroup_size(16,16,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let z = global_id.y;
    if true {
        let i = atomicAdd(&instance_count, 1u);
        let hash = hash_2d(x, z);

        instances[i].hash = hash;
        instances[i].pos = vec3<f32>(
            (f32(x) / blades_per_side) * tile_size,
            0.0,
            (f32(z) / blades_per_side) * tile_size,
        );

        instances[i].wind = hash_to_range(hash) * PI / 6.0;
        instances[i].facing = normalize(hash_to_vec2_neg(hash)) * 2.0 * PI;
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

// generates float in range [0, 1]
fn hash_to_range_neg(hash: u32) -> f32 {
    return (f32(hash) * 2.3283064e-10) * 2.0 - 1.0; // hash * 1 / 2^32
}

// generates vec2 with values in range [-1, 1]
//fn hash_to_vec(hash: u32) -> vec2<f32> {
//    var float1: f32 = hash_to_range(hash ^ 0x36753621u);
//    var float2: f32 = hash_to_range(hash ^ 0x12345678u);
//    return vec2<f32>(
//        float1 * 2.0 - 1.0,
//        float2 * 2.0 - 1.0,
//    );
//}

//fn hash(input: u32) -> u32 {
//    let state = input * u32(747796405) + u32(2891336453u);
//    let word = ((state >> ((state >> u32(28)) + u32(4))) ^ state) * u32(277803737);
//    return (word >> u32(22)) ^ word;
//}