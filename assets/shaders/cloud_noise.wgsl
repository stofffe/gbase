@group(0) @binding(0) var<uniform> noise_info : NoiseGeneratorInfo;
struct NoiseGeneratorInfo {
    size: u32,
    cells: u32,
};
@group(0) @binding(1) var output :  texture_storage_3d<rgba8unorm, write>;

const POINT_OFFSET_MULT = 0.7;

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = i32(noise_info.size);
    let cell_size = i32(noise_info.size / noise_info.cells);
    let cells = i32(noise_info.cells);
    let center_cell = (vec3<i32>(id) / cell_size);
    let pixel_pos = vec3<i32>(id);

    var closest_dist = f32(noise_info.size);
    var closest_hash = 0u;
    var closest_pos = vec3<f32>(0.0, 0.0, 0.0);

    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            for (var k = -1; k <= 1; k++) {
                let step = vec3<i32>(i, j, k);
                let cell = (center_cell + step + cells) % cells; // [0, cells]

                let cell_hash = hash_3d(u32(cell.x), u32(cell.y), u32(cell.z));
                let cell_pos = vec3<f32>(cell * cell_size) + f32(cell_size / 2);
                var cell_point_pos = cell_pos + hash_to_vec3_snorm(cell_hash) * f32(cell_size) * POINT_OFFSET_MULT;

                // handle wrapping
                var wrapped_pixel_pos = vec3<f32>(pixel_pos);
                let dir = vec3<f32>(step * cell_size);
                wrapped_pixel_pos += dir;
                wrapped_pixel_pos = (wrapped_pixel_pos + f32(size)) % f32(size);
                wrapped_pixel_pos -= dir;

                let dist = length(vec3<f32>(wrapped_pixel_pos) - cell_point_pos);
                if dist < closest_dist {
                    closest_dist = dist;
                    closest_pos = cell_point_pos;
                    closest_hash = cell_hash;
                }
            }
        }
    }

    var color: vec3<f32>;
    color = hash_to_vec4_unorm(closest_hash).xyz;
    color = 1.0 - vec3<f32>(closest_dist, closest_dist, closest_dist) / f32(cell_size);
    textureStore(output, id, vec4<f32>(color, 1.0));
}

const SEED = 0x22314u;
const FLOAT_SCALE = 1.0 / f32(0xffffffffu);

fn hash_1d(x: u32) -> u32 {
    var hash: u32 = x ^ SEED;
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
    return hash;
}


fn hash_2d(x: u32, y: u32) -> u32 {
    // Use Wang hash for better distribution
    var hash: u32 = x ^ SEED;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
    return hash;
}

fn hash_3d(x: u32, y: u32, z: u32) -> u32 {
    var hash: u32 = x ^ SEED;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash ^ ((z << 11u) | (z >> 21u));  // Different shift for z to avoid patterns
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
