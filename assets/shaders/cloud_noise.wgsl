@group(0) @binding(0) var<uniform> noise_info : NoiseGeneratorInfo;
struct NoiseGeneratorInfo {
    size: u32,
    cells: u32,
};
@group(0) @binding(1) var output :  texture_storage_2d<rgba8unorm, write>;

const POINT_OFFSET_MULT = 0.5;

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cell_size = i32(noise_info.size / noise_info.cells);
    let cells = i32(noise_info.cells);
    let center_cell = vec2<i32>(id.xy) / cell_size;
    let pixel_pos = vec2<i32>(id.xy);

    var closest_dist = f32(noise_info.size);
    var closest_hash = 0u;
    var closest_pos = vec2<f32>(0.0, 0.0);

    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let cell = center_cell + vec2<i32>(i, j); // [0, cells]

            let cell_hash = hash_2d_i32(cell.x, cell.y);
            let cell_pos = vec2<f32>(cell * cell_size);
            let cell_point_pos = cell_pos + hash_to_vec2_snorm(cell_hash) * f32(cell_size) * POINT_OFFSET_MULT;

            let dist = length(vec2<f32>(pixel_pos) - cell_point_pos);
            if dist < closest_dist {
                closest_dist = dist;
                closest_pos = cell_point_pos;
                closest_hash = cell_hash;
            }
        }
    }

    let color = hash_to_vec4(closest_hash);
    textureStore(output, id.xy, vec4<f32>(color.xyz, 1.0));
    //textureStore(output, id.xy, vec4<f32>(cell_pos.xy, 0.0, 1.0));
}

    //            float3 p = Points[Index(xid, yid, zid)] + offset;
    //            float dist = length(pixelPos - p);
    //            minDist = min(minDist, dist);






    //uint cellSize = Size / Cells;
    //uint x = id.x; uint y = id.y; uint z = id.z;
    //uint cx = x / cellSize; uint cy = y / cellSize; uint cz = z / cellSize;

    //float3 pixelPos = float3((float)x, (float)y, (float)z);
    //float minDist = (float)Size;

    //for (int i = -1; i <= 1; i++) {
    //    for (int j = -1; j <= 1; j++) {
    //        for (int k = -1; k <= 1; k++) {
    //            int xid = (cx + i + Cells) % Cells;
    //            int yid = (cy + j + Cells) % Cells;
    //            int zid = (cz + k + Cells) % Cells;

    //            // Tiling offset
    //            float3 offset = float3(0.0, 0.0, 0.0);
    //            if (cx + i == -1) { offset.x -= Size; }
    //            if (cx + i == Cells) { offset.x += Size; }
    //            if (cy + j == -1) { offset.y -= Size; }
    //            if (cy + j == Cells) { offset.y += Size; }
    //            if (cz + k == -1) { offset.z -= Size; }
    //            if (cz + k == Cells) { offset.z += Size; }

    //            float3 p = Points[Index(xid, yid, zid)] + offset;
    //            float dist = length(pixelPos - p);
    //            minDist = min(minDist, dist);
    //        }
    //    }
    //}

    //float invertedDist = 1.0 - (minDist / cellSize);
    //uint pixelIndex = id.x + id.y * Size + id.z * Size * Size;
    //Result[pixelIndex] = invertedDist;

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
fn hash_2d_i32(x: i32, y: i32) -> u32 {
    // Use Wang hash for better distribution
    var hash: i32 = x;
    hash = hash ^ ((y << 16) | (y >> 16));
    hash = hash * 0x85ebca6;
    hash = hash ^ (hash >> 13);
    hash = hash * 0xc2b2ae3;
    hash = hash ^ (hash >> 16);
    return u32(hash);
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
