@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var prev: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

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
    let blur = sample_9tap(in.uv);
    let prev_color = textureSample(prev, samp, in.uv).rgb;
    let color = blur + prev_color;
    return vec4f(color, 1.0);
}

fn sample_9tap(uv: vec2f) -> vec3f {
    let tex_dim = textureDimensions(tex, 0);
    let p = 1.0 * vec2f(1.0 / f32(tex_dim.x), 1.0 / f32(tex_dim.y));

    var color = vec3f(0.0);
    color += textureSample(tex, samp, uv + vec2f(-p.x, -p.y)).rgb * 1.0;
    color += textureSample(tex, samp, uv + vec2f(0.0, -p.y)).rgb * 2.0;
    color += textureSample(tex, samp, uv + vec2f(p.x, -p.y)).rgb * 1.0;
    color += textureSample(tex, samp, uv + vec2f(-p.x, 0.0)).rgb * 2.0;
    color += textureSample(tex, samp, uv + vec2f(0.0, 0.0)).rgb * 4.0;
    color += textureSample(tex, samp, uv + vec2f(p.x, 0.0)).rgb * 2.0;
    color += textureSample(tex, samp, uv + vec2f(-p.x, p.y)).rgb * 1.0;
    color += textureSample(tex, samp, uv + vec2f(0.0, p.y)).rgb * 2.0;
    color += textureSample(tex, samp, uv + vec2f(p.x, p.y)).rgb * 1.0;
    color *= 0.0625; // 1/16

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
