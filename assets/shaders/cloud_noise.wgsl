@group(0) @binding(0) var<uniform> settings : NoiseGeneratorInfo;
struct NoiseGeneratorInfo {
    size: u32,
    cells_r: u32,
    cells_g: u32,
    cells_b: u32,
    cells_a: u32,
};
@group(0) @binding(1) var output :  texture_storage_3d<rgba8unorm, write>;

const POINT_OFFSET_MULT = 0.5;

// TODO: are f32s even needed?

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let worley1 = generate_worley_noise_3d(settings.size, settings.cells_r, id);
    let worley2 = generate_worley_noise_3d(settings.size, settings.cells_g, id);
    let worley3 = generate_worley_noise_3d(settings.size, settings.cells_b, id);
    let worley4 = generate_worley_noise_3d(settings.size, settings.cells_a, id);

    let color = vec4<f32>(worley1, worley2, worley3, worley4);
    textureStore(output, id, color);
}

// generate worley
fn generate_worley_noise_3d(size: u32, cells: u32, coord: vec3<u32>) -> f32 {
    let cell_size = i32(size / cells);
    let center_cell = vec3<i32>(coord) / cell_size;
    let pixel_pos = vec3<i32>(coord);

    var closest_dist = f32(size);
    var closest_hash = 0u;

    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            for (var k = -1; k <= 1; k++) {
                let cell_index = center_cell + vec3<i32>(i, j, k);

                let cell_index_wrapped = mod_vec3_i32(cell_index, i32(cells));
                let cell_hash = hash_3d(u32(cell_index_wrapped.x), u32(cell_index_wrapped.y), u32(cell_index_wrapped.z));

                let cell_pos = vec3<f32>(cell_index * cell_size) + f32(cell_size / 2);
                let cell_pos_jittered = cell_pos + hash_to_vec3_snorm(cell_hash) * f32(cell_size) * POINT_OFFSET_MULT;

                let dist = length(vec3<f32>(pixel_pos) - cell_pos_jittered);
                if dist < closest_dist {
                    closest_dist = dist;
                    closest_hash = cell_hash;
                }
            }
        }
    }

    return 1.0 - closest_dist / f32(cell_size);
}

//
// utils
//

//
// modulo functions with support for negative values
//

fn mod_f32(value: f32, n: f32) -> f32 { return (value + n) % n; }
fn mod_vec2_f32(value: vec2<f32>, n: f32) -> vec2<f32> { return (value + n) % n; }
fn mod_vec3_f32(value: vec3<f32>, n: f32) -> vec3<f32> { return (value + n) % n; }
fn mod_vec4_f32(value: vec4<f32>, n: f32) -> vec4<f32> { return (value + n) % n; }

fn mod_i32(value: i32, n: i32) -> i32 { return (value + n) % n; }
fn mod_vec2_i32(value: vec2<i32>, n: i32) -> vec2<i32> { return (value + n) % n; }
fn mod_vec3_i32(value: vec3<i32>, n: i32) -> vec3<i32> { return (value + n) % n; }
fn mod_vec4_i32(value: vec4<i32>, n: i32) -> vec4<i32> { return (value + n) % n; }

fn mod_u32(value: u32, n: u32) -> u32 { return (value + n) % n; }
fn mod_vec2_u32(value: vec2<u32>, n: u32) -> vec2<u32> { return (value + n) % n; }
fn mod_vec3_u32(value: vec3<u32>, n: u32) -> vec3<u32> { return (value + n) % n; }
fn mod_vec4_u32(value: vec4<u32>, n: u32) -> vec4<u32> { return (value + n) % n; }

//
// random number generation
//

const SEED = 0x2231414u;

// generate hash from singe u32
fn hash_1d(x: u32) -> u32 {
    var hash: u32 = x ^ SEED;
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
    return hash;
}

// generate hash from two u32
fn hash_2d(x: u32, y: u32) -> u32 {
    var hash: u32 = x ^ SEED;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
    return hash;
}

// generate hash from three u32
fn hash_3d(x: u32, y: u32, z: u32) -> u32 {
    var hash: u32 = x ^ SEED;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash ^ ((z << 11u) | (z >> 21u));
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

// generates vec2 with values in range [-1, 1]
fn hash_to_vec2_unorm(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_unorm(hash_2d(hash, 0xB5297A4Du)),
        hash_to_unorm(hash_2d(hash, 0x68E31DA4u)),
    );
}

// generates vec2 with values in range [-1, 1]
fn hash_to_vec2_snorm(hash: u32) -> vec2<f32> {
    return vec2<f32>(
        hash_to_snorm(hash_2d(hash, 0xB5297A4Du)),
        hash_to_snorm(hash_2d(hash, 0x68E31DA4u)),
    );
}

// generates vec3 with values in range [-1, 1]
fn hash_to_vec3_unorm(hash: u32) -> vec3<f32> {
    return vec3<f32>(
        hash_to_unorm(hash_2d(hash, 0xB5297A4Du)),
        hash_to_unorm(hash_2d(hash, 0x68E31DA4u)),
        hash_to_unorm(hash_2d(hash, 0x1B56C4E9u)),
    );
}

// generates vec3 with values in range [-1, 1]
fn hash_to_vec3_snorm(hash: u32) -> vec3<f32> {
    return vec3<f32>(
        hash_to_snorm(hash_2d(hash, 0xB5297A4Du)),
        hash_to_snorm(hash_2d(hash, 0x68E31DA4u)),
        hash_to_snorm(hash_2d(hash, 0x1B56C4E9u)),
    );
}

// generates vec3 with values in range [-1, 1]
fn hash_to_vec4_unorm(hash: u32) -> vec4<f32> {
    return vec4<f32>(
        hash_to_unorm(hash_2d(hash, 0xB5297A4Du)),
        hash_to_unorm(hash_2d(hash, 0x68E31DA4u)),
        hash_to_unorm(hash_2d(hash, 0x1B56C4E9u)),
        hash_to_unorm(hash_2d(hash, 0xA341316Cu)),
    );
}

// generates vec3 with values in range [-1, 1]
fn hash_to_vec4_snorm(hash: u32) -> vec4<f32> {
    return vec4<f32>(
        hash_to_snorm(hash_2d(hash, 0xB5297A4Du)),
        hash_to_snorm(hash_2d(hash, 0x68E31DA4u)),
        hash_to_snorm(hash_2d(hash, 0x1B56C4E9u)),
        hash_to_snorm(hash_2d(hash, 0xA341316Cu)),
    );
}
