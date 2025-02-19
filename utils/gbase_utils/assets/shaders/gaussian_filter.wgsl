@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    kernel_size: i32, // side length (3 => 3x3 matrix)
    sigma: f32,
}

const PI: f32 = 3.14159265359;

fn gaussian_1(x: f32, sigma: f32) -> f32 {
    let sigma2: f32 = sigma * sigma;
    let exponent: f32 = -(x * x) / (2.0 * sigma2);
    return (1.0 / sqrt(2.0 * PI * sigma2)) * exp(exponent);
}
fn gaussian_2(x: f32, y: f32, sigma: f32) -> f32 {
    let sigma2: f32 = sigma * sigma;
    let exponent: f32 = -(x * x + y * y) / (2.0 * sigma2);
    return (1.0 / (2.0 * PI * sigma2)) * exp(exponent);
}

@compute @workgroup_size(1, 1, 1)
fn horizontal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    let ksize = params.kernel_size; // kernel size (half the side length)
    let dim = vec2<i32>(textureDimensions(in_texture));

    var sum: vec4<f32> = vec4<f32>(0.0);
    var weight_sum: f32 = 0.0;

    for (var i = -ksize; i <= ksize; i++) {
        let sample_pos = vec2<i32>(x + i, y);
        if sample_pos.x >= 0 && sample_pos.x < dim.x {
            let weight: f32 = gaussian_1(f32(i), params.sigma);
            let tex_value: vec4<f32> = textureLoad(in_texture, sample_pos, 0);
            sum += tex_value * weight;
            weight_sum += weight;
        }
    }
    sum /= weight_sum;
    textureStore(out_texture, global_id.xy, sum);
}

@compute @workgroup_size(1, 1, 1)
fn vertical(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    let ksize = params.kernel_size; // kernel size (half the side length)
    let dim = vec2<i32>(textureDimensions(in_texture));

    var sum: vec4<f32> = vec4<f32>(0.0);
    var weight_sum: f32 = 0.0;

    for (var i = -ksize; i <= ksize; i++) {
        let sample_pos = vec2<i32>(x, y + i);
        if sample_pos.y >= 0 && sample_pos.y < dim.y {
            let weight: f32 = gaussian_1(f32(i), params.sigma);
            let tex_value: vec4<f32> = textureLoad(in_texture, sample_pos, 0);
            sum += tex_value * weight;
            weight_sum += weight;
        }
    }
    sum /= weight_sum;
    textureStore(out_texture, global_id.xy, sum);
}

@compute @workgroup_size(1, 1, 1)
fn horizontal_vertical(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    let ksize = params.kernel_size; // kernel size (half the side length)
    let dim = vec2<i32>(textureDimensions(in_texture));

    var sum: vec4<f32> = vec4<f32>(0.0);
    var weight_sum: f32 = 0.0;
    for (var i = -ksize; i <= ksize; i++) {
        for (var j = -ksize; j <= ksize; j++) {
            let sample_pos = vec2<i32>(x + i, y + j);
            if sample_pos.x >= 0 && sample_pos.x < dim.x && sample_pos.y >= 0 && sample_pos.y < dim.y {
                let weight: f32 = gaussian_2(f32(i), f32(j), params.sigma);
                let tex_value: vec4<f32> = textureLoad(in_texture, sample_pos, 0);
                sum += tex_value * weight;
                weight_sum += weight;
            }
        }
    }
    sum /= weight_sum;
    textureStore(out_texture, global_id.xy, sum);
}
