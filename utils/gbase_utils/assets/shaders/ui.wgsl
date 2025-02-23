diagnostic (off, derivative_uniformity);

struct VertexInput {
    // vertex attr
    @location(0) position: vec3f,
    @location(1) uv: vec2f,

    // instance attr
    @location(2) top_left_pos: vec2f,
    @location(3) scale: vec2f,
    @location(4) atlas_offset: vec2f,
    @location(5) atlas_scale: vec2f,
    @location(6) color: vec4f,
    @location(7) @interpolate(flat) ty: u32,
    @location(8) border_radius: vec4f,
}

const TYPE_SHAPE = 0u;
const TYPE_TEXT = 1u;
const EDGE_SOFTNESS = 1.5;

@group(0) @binding(0) var font_tex: texture_2d<f32>;
@group(0) @binding(1) var font_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: CameraUniform;
@group(0) @binding(3) var<uniform> app_info: AppInfo;

struct CameraUniform {
    pos: vec3<f32>,
    facing: vec3<f32>,

    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,

    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}

struct AppInfo {
    t: f32,
    screen_width: u32,
    screen_height: u32,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = (in.scale * in.position.xy) + in.top_left_pos * vec2f(1.0, -1.0);
    out.world_pos = world_pos;
    out.scale = in.scale;
    out.center = (in.top_left_pos + in.scale * 0.5) * vec2f(1.0, -1.0);
    out.uv = in.uv;
    out.atlas_uv = in.uv * in.atlas_scale + in.atlas_offset;
    out.color = in.color;
    out.ty = in.ty;
    out.border_radius = in.border_radius;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);

    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) world_pos: vec2f,
    @location(1) scale: vec2f,
    @location(2) center: vec2f,
    @location(3) uv: vec2f,
    @location(4) atlas_uv: vec2f,
    @location(5) color: vec4f,
    @location(6) @interpolate(flat) ty: u32,
    @location(7) border_radius: vec4f,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var alpha = 1.0;

    if in.ty == TYPE_TEXT {
        // font lookup
        alpha = min(alpha, textureSample(font_tex, font_sampler, in.atlas_uv).x);
    }

    if in.ty == TYPE_SHAPE {
        // rounded borders
        let dist = sdf_box_rounded(in.world_pos - in.center, in.scale / 2.0, in.border_radius);
        let edge_alpha = 1.0 - smoothstep(0.0, EDGE_SOFTNESS, dist);
        alpha = min(alpha, edge_alpha);

        if alpha <= 0.0 {
            discard;
        }
    }

    return vec4f(in.color.xyz, alpha);
}

//
// Utils
//

fn sdf_circle(point: vec2f, radius: f32) -> f32 {
    return length(point) - radius;
}

// x: tr
// y: br
// z: tl
// w: bl

// x: tr
// z: tl
// y: br
// w: bl

// fn sdf_box_rounded(point: vec2f, extents: vec2f, radius: vec4f) -> f32 {
//     var r = select(radius.zw, radius.xy, point.x > 0.0); // Choose between left/right radii based on x
//     r.x = select(r.y, r.x, point.y > 0.0); // Choose between top/bottom based on y
//     let q = abs(point) - extents + r.x;
//     return min(max(q.x, q.y), 0.0) + length(max(q, vec2f(0.0))) - r.x;
// }

fn sdf_box_rounded(point: vec2f, extents: vec2f, radius: vec4f) -> f32 {
    var r = select(radius.xw, radius.yz, point.x > 0.0); // Choose between left/right radii based on x
    r.x = select(r.y, r.x, point.y > 0.0); // Choose between top/bottom based on y
    let q = abs(point) - extents + r.x;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2f(0.0))) - r.x;
}
