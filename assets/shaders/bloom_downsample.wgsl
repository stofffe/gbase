@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// Vertex shader

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4f(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var color = sample_13tap(in.uv);
    return vec4f(color, 1.0);
}

fn sample_13tap(uv: vec2f) -> vec3f {
    let tex_dim = textureDimensions(tex, 0);
    let p = 1.0 * vec2f(1.0 / f32(tex_dim.x), 1.0 / f32(tex_dim.y));

    let a = textureSample(tex, samp, uv + vec2f(-2.0 * p.x, -2.0 * p.y)).rgb;
    let b = textureSample(tex, samp, uv + vec2f(0.0, -2.0 * p.y)).rgb;
    let c = textureSample(tex, samp, uv + vec2f(2.0 * p.x, -2.0 * p.y)).rgb;
    let d = textureSample(tex, samp, uv + vec2f(-p.x, -p.y)).rgb;
    let e = textureSample(tex, samp, uv + vec2f(p.x, -p.y)).rgb;
    let f = textureSample(tex, samp, uv + vec2f(-2.0 * p.x, 0.0)).rgb;
    let g = textureSample(tex, samp, uv + vec2f(0.0, 0.0)).rgb;
    let h = textureSample(tex, samp, uv + vec2f(2.0 * p.x, 0.0)).rgb;
    let i = textureSample(tex, samp, uv + vec2f(-p.x, p.y)).rgb;
    let j = textureSample(tex, samp, uv + vec2f(p.x, p.y)).rgb;
    let k = textureSample(tex, samp, uv + vec2f(-2.0 * p.x, 2.0 * p.y)).rgb;
    let l = textureSample(tex, samp, uv + vec2f(0.0, 2.0 * p.y)).rgb;
    let m = textureSample(tex, samp, uv + vec2f(2.0 * p.x, 2.0 * p.y)).rgb;

    let tl = (a + b + f + g) * 0.25;
    let tr = (b + c + g + h) * 0.25;
    let bl = (f + g + k + l) * 0.25;
    let br = (g + h + l + m) * 0.25;
    let mid = (d + e + i + j) * 0.25;

    let color = tl * 0.125 + tr * 0.125 + bl * 0.125 + br * 0.125 + mid * 0.5;

    return color;
}

fn sample_4tap(uv: vec2f) -> vec3f {
    let tex_dim = textureDimensions(tex, 0);
    let texel_uv = 1.0 * vec2f(1.0 / f32(tex_dim.x), 1.0 / f32(tex_dim.y));

    var color = vec3f(0.0);
    color += textureSample(tex, samp, uv + vec2f(-texel_uv.x, -texel_uv.y)).rgb;
    color += textureSample(tex, samp, uv + vec2f(texel_uv.x, -texel_uv.y)).rgb;
    color += textureSample(tex, samp, uv + vec2f(-texel_uv.x, texel_uv.y)).rgb;
    color += textureSample(tex, samp, uv + vec2f(texel_uv.x, texel_uv.y)).rgb;
    color *= 0.25;

    return color;
}
