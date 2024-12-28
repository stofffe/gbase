@group(0) @binding(0) var<uniform> noise_info : NoiseGeneratorInfo;
struct NoiseGeneratorInfo {
    size: u32,
    cells: u32,
};
@group(0) @binding(1) var output :  texture_storage_2d<rgba8unorm, write>;

const POINT_OFFSET_MULT = 0.7;

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = i32(noise_info.size);
    let cell_size = i32(noise_info.size / noise_info.cells);
    let cells = i32(noise_info.cells);
    let center_cell = (vec2<i32>(id.xy) / cell_size);
    let pixel_pos = vec2<i32>(id.xy);

    var closest_dist = f32(noise_info.size);
    var closest_hash = 0u;
    var closest_pos = vec2<f32>(0.0, 0.0);

    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let step = vec2<i32>(i, j);
            let cell = (center_cell + step + cells) % cells; // [0, cells]

            let cell_hash = hash_2d(u32(cell.x), u32(cell.y));
            let cell_pos = vec2<f32>(cell * cell_size) + f32(cell_size / 2);
            var cell_point_pos = cell_pos + hash_to_vec2_snorm(cell_hash) * f32(cell_size) * POINT_OFFSET_MULT;

            // handle wrapping
            if i == 1 && cell.x == 0 { cell_point_pos.x += f32(size); }
            if j == 1 && cell.y == 0 { cell_point_pos.y += f32(size); }
            if i == -1 && cell.x == cells - 1 { cell_point_pos.x -= f32(size); }
            if j == -1 && cell.y == cells - 1 { cell_point_pos.y -= f32(size); }

            let dist = length(vec2<f32>(pixel_pos) - cell_point_pos);
            if dist < closest_dist {
                closest_dist = dist;
                closest_pos = cell_point_pos;
                closest_hash = cell_hash;
            }
        }
    }

    var color: vec3<f32>;
    color = hash_to_vec4(closest_hash).xyz;
    color = 1.0 - vec3<f32>(closest_dist, closest_dist, closest_dist) / f32(cell_size);
    textureStore(output, id.xy, vec4<f32>(color, 1.0));
}

const SEED = 0x22314u;
// generates hash from two u32:s
fn hash_2d(x: u32, y: u32) -> u32 {
    // Use Wang hash for better distribution
    var hash: u32 = x ^ SEED;
    hash = hash ^ ((y << 16u) | (y >> 16u));
    hash = hash * 0x85ebca6bu;
    hash = hash ^ (hash >> 13u);
    hash = hash * 0xc2b2ae35u;
    hash = hash ^ (hash >> 16u);
    return hash ^ SEED;
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

fn hash_to_vec4(hash: u32) -> vec4<f32> {
    return vec4<f32>(
        hash_to_unorm(hash ^ 0x36753621u),
        hash_to_unorm(hash ^ 0x12345678u),
        hash_to_unorm(hash ^ 0x43284732u),
        hash_to_unorm(hash ^ 0x91273127u),
    );
}
