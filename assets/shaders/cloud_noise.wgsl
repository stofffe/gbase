@group(0) @binding(0) var<uniform> settings: NoiseGeneratorInfo;
struct NoiseGeneratorInfo {
    size: u32,
    cells_r: u32,
    cells_g: u32,
    cells_b: u32,
    cells_a: u32,
}
@group(0) @binding(1) var output: texture_storage_3d<rgba8unorm, write>;

const POINT_OFFSET_MULT = 0.5;

// TODO: are f32s even needed?

@compute @workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let worley1 = worley_3d(id, settings.size, settings.cells_r);
    let worley2 = worley_3d(id, settings.size, settings.cells_g);
    let worley3 = worley_3d(id, settings.size, settings.cells_b);

    // let perlin1 = perlinFBM(vec3f(id) / f32(settings.size));
    // let perlin1 = fbm2d(vec3f(id).xy / f32(settings.size));
    let perlin1 = perlin_fbm_3d(vec3f(id) / f32(settings.size), 10.0);

    let color = vec4<f32>(worley1, worley2, worley3, perlin1);
    textureStore(output, id, color);
}

// generate worley
fn worley_3d(coord: vec3<u32>, size: u32, cells: u32) -> f32 {
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

fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    return to_min + (to_max - to_min) * ((value - from_min) / (from_max - from_min));
}

//
// modulo functions with support for negative values
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

//
// PERLIN
//

fn hash3d(pos: vec3f) -> f32 {
    return fract(sin(dot(pos, vec3<f32>(27.16898, 38.90563, 43.23476))) * 5151.5473453);
}

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

    var scale = noise_scale * 3.0;
    var amp = 0.5;
    for (var i = 0; i < 5; i = i + 1) {
        f += perlin_3d(p, scale) * amp;
        amp = amp * 0.5;
        scale = scale * 2.0;
    }

    return f;
// return (f + 1.0) * 0.5;
}

// noise(s) * a +
// noise(2 s) + 0.5 a +
// noise(4s) + 0.25 a+
// noise(8s) + 0.125 a+

// fn perlinFBM(pos: vec3f) -> f32 {
//     let coord = pos;
//     let p1 = perlinNoise3(coord * 5.0);
//     let p2 = perlinNoise3(coord * 10.0);
//     let p3 = perlinNoise3(coord * 15.0);
//     let p = p1 * 0.625 + p2 * 0.25 + p3 * 0.125;
//     return (p + 1.0) * 0.5;
// }
//
// fn permute4(x: vec4f) -> vec4f { return ((x * 34. + 1.) * x) % vec4f(289.); }
// fn taylorInvSqrt4(r: vec4f) -> vec4f { return 1.79284291400159 - 0.85373472095314 * r; }
// fn fade3(t: vec3f) -> vec3f { return t * t * t * (t * (t * 6. - 15.) + 10.); }
//
// fn perlinNoise3(P: vec3f) -> f32 {
//     var Pi0: vec3f = floor(P); // Integer part for indexing
//     var Pi1: vec3f = Pi0 + vec3f(1.); // Integer part + 1
//     Pi0 = Pi0 % vec3f(289.);
//     Pi1 = Pi1 % vec3f(289.);
//     let Pf0 = fract(P); // Fractional part for interpolation
//     let Pf1 = Pf0 - vec3f(1.); // Fractional part - 1.
//     let ix = vec4f(Pi0.x, Pi1.x, Pi0.x, Pi1.x);
//     let iy = vec4f(Pi0.yy, Pi1.yy);
//     let iz0 = vec4f(1.0) * Pi0.z;
//     let iz1 = vec4f(1.0) * Pi1.z;
//
//     let ixy = permute4(permute4(ix) + iy);
//     let ixy0 = permute4(ixy + iz0);
//     let ixy1 = permute4(ixy + iz1);
//
//     var gx0: vec4f = ixy0 / 7.;
//     var gy0: vec4f = fract(floor(gx0) / 7.) - 0.5;
//     gx0 = fract(gx0);
//     var gz0: vec4f = vec4f(0.5) - abs(gx0) - abs(gy0);
//     var sz0: vec4f = step(gz0, vec4f(0.));
//     gx0 = gx0 + sz0 * (step(vec4f(0.), gx0) - 0.5);
//     gy0 = gy0 + sz0 * (step(vec4f(0.), gy0) - 0.5);
//
//     var gx1: vec4f = ixy1 / 7.;
//     var gy1: vec4f = fract(floor(gx1) / 7.) - 0.5;
//     gx1 = fract(gx1);
//     var gz1: vec4f = vec4f(0.5) - abs(gx1) - abs(gy1);
//     var sz1: vec4f = step(gz1, vec4f(0.));
//     gx1 = gx1 - sz1 * (step(vec4f(0.), gx1) - 0.5);
//     gy1 = gy1 - sz1 * (step(vec4f(0.), gy1) - 0.5);
//
//     var g000: vec3f = vec3f(gx0.x, gy0.x, gz0.x);
//     var g100: vec3f = vec3f(gx0.y, gy0.y, gz0.y);
//     var g010: vec3f = vec3f(gx0.z, gy0.z, gz0.z);
//     var g110: vec3f = vec3f(gx0.w, gy0.w, gz0.w);
//     var g001: vec3f = vec3f(gx1.x, gy1.x, gz1.x);
//     var g101: vec3f = vec3f(gx1.y, gy1.y, gz1.y);
//     var g011: vec3f = vec3f(gx1.z, gy1.z, gz1.z);
//     var g111: vec3f = vec3f(gx1.w, gy1.w, gz1.w);
//
//     let norm0 = taylorInvSqrt4(vec4f(dot(g000, g000), dot(g010, g010), dot(g100, g100), dot(g110, g110)));
//     g000 = g000 * norm0.x;
//     g010 = g010 * norm0.y;
//     g100 = g100 * norm0.z;
//     g110 = g110 * norm0.w;
//     let norm1 = taylorInvSqrt4(vec4f(dot(g001, g001), dot(g011, g011), dot(g101, g101), dot(g111, g111)));
//     g001 = g001 * norm1.x;
//     g011 = g011 * norm1.y;
//     g101 = g101 * norm1.z;
//     g111 = g111 * norm1.w;
//
//     let n000 = dot(g000, Pf0);
//     let n100 = dot(g100, vec3f(Pf1.x, Pf0.yz));
//     let n010 = dot(g010, vec3f(Pf0.x, Pf1.y, Pf0.z));
//     let n110 = dot(g110, vec3f(Pf1.xy, Pf0.z));
//     let n001 = dot(g001, vec3f(Pf0.xy, Pf1.z));
//     let n101 = dot(g101, vec3f(Pf1.x, Pf0.y, Pf1.z));
//     let n011 = dot(g011, vec3f(Pf0.x, Pf1.yz));
//     let n111 = dot(g111, Pf1);
//
//     var fade_xyz: vec3f = fade3(Pf0);
//     let temp = vec4f(f32(fade_xyz.z)); // simplify after chrome bug fix
//     let n_z = mix(vec4f(n000, n100, n010, n110), vec4f(n001, n101, n011, n111), temp);
//     let n_yz = mix(n_z.xy, n_z.zw, vec2f(f32(fade_xyz.y))); // simplify after chrome bug fix
//     let n_xyz = mix(n_yz.x, n_yz.y, fade_xyz.x);
//     return 2.2 * n_xyz;
// }
fn dontremovecommentspls() { }
