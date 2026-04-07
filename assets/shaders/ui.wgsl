diagnostic (off, derivative_uniformity);

@group(0) @binding(0) var<uniform> projection: mat4x4f;
@group(0) @binding(1) var font_atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var font_atlas_sampler: sampler;

struct VertexInput {
    // instance
    @location(0) position: vec2f,
    @location(1) size: vec2f,
    @location(2) color: vec4f,

    @location(3) font_atlas_offset: vec2f,
    @location(4) font_atlas_size: vec2f,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Generate quad corner 0..1
    let uv = vec2f(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );

    let pixel_pos = in.position + (uv) * in.size;

    let position = projection * vec4f(pixel_pos, 0.0, 1.0);

    out.clip_position = position;
    out.uv = uv;
    out.color = in.color;

    let atlas_offset = in.font_atlas_offset;
    let atlas_size = in.font_atlas_size;

    out.atlas_uv = atlas_offset + atlas_size * uv;

    return out;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
    @location(2) atlas_uv: vec2f,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let font_sdf = textureSample(font_atlas_texture, font_atlas_sampler, in.atlas_uv);
    if !(in.atlas_uv.x == 0.0 && in.atlas_uv.y == 0) {
        if font_sdf.r > 0.5 {
            let dist = font_sdf.r;
            let edge = 0.5;
            // let w = fwidth(dist);
            // let alpha = smoothstep(edge - w, edge + w, dist);
            let alpha = smoothstep(0.40, 0.60, dist);
            return vec4f(alpha, alpha, alpha, 1.0);
        // let color = smoothstep()
        // return vec4f(1.0, 1.0, 1.0, 1.0);
        } else {
            discard;
        }
    }

    return vec4f(in.color);
}
