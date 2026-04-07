diagnostic (off, derivative_uniformity);

@group(0) @binding(0) var<uniform> projection: mat4x4f;
@group(0) @binding(1) var font_atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var font_atlas_sampler: sampler;

struct VertexInput {
    // instance
    @location(0) @interpolate(flat) element_type: u32,

    @location(1) position: vec2f,
    @location(2) size: vec2f,
    @location(3) color: vec4f,

    @location(4) font_atlas_offset: vec2f,
    @location(5) font_atlas_size: vec2f,
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

    out.element_type = in.element_type;
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
    @location(0) @interpolate(flat) element_type: u32,
    @location(1) uv: vec2f,
    @location(2) color: vec4f,
    @location(3) atlas_uv: vec2f,
}

const ELEMENT_TYPE_CONTAINER = 0u;
const ELEMENT_TYPE_GLYPH = 1u;

const EDGE_CUTOFF = 0.5;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let font_sdf = textureSample(font_atlas_texture, font_atlas_sampler, in.atlas_uv);

    if in.element_type == ELEMENT_TYPE_CONTAINER {
        return vec4f(in.color);
    } else if in.element_type == ELEMENT_TYPE_GLYPH {
        let dist = font_sdf.r;
        let w = fwidth(dist);
        let glyph_alpha = smoothstep(EDGE_CUTOFF - w, EDGE_CUTOFF + w, dist);
        let alpha = min(glyph_alpha, in.color.a);
        return vec4f(in.color.rgb, alpha);
    } else {
        return vec4f(1.0, 0.0, 1.0, 1.0);
    }

}
