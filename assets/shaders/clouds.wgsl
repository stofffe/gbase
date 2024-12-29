struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> app_info: AppInfo;
struct AppInfo {
    t: f32,
    screen_width: u32,
    screen_height: u32,
};

@group(0) @binding(1) var<uniform> camera: CameraUniform;
struct CameraUniform {
    pos: vec3<f32>,
    facing: vec3<f32>,
    
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,

    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
};
@group(0) @binding(2) var<uniform> bounding_box: BoundingBox;
struct BoundingBox {
    origin: vec3<f32>,
    dimensions: vec3<f32>,
};
@group(0) @binding(3) var noise_tex: texture_3d<f32>;
@group(0) @binding(4) var noise_samp: sampler;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv * 2.0;
    var z = (app_info.t / 10.0) % 1.0;

    let coord = vec3<f32>(uv.x, uv.y, z);

    let value = textureSample(noise_tex, noise_samp, coord).a;
    let color = vec4<f32>(value);
    return vec4<f32>(color);
}
