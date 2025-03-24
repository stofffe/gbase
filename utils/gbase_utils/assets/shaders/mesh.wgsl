struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> model: mat4x4f;
@group(0) @binding(2) var base_color_texture: texture_2d<f32>;
@group(0) @binding(3) var base_color_sampler: sampler;
@group(0) @binding(4) var normal_texture: texture_2d<f32>;
@group(0) @binding(5) var normal_sampler: sampler;
@group(0) @binding(6) var metallic_roughness_texture: texture_2d<f32>;
@group(0) @binding(7) var metallic_roughness_sampler: sampler;
@group(0) @binding(8) var occlusion_texture: texture_2d<f32>;
@group(0) @binding(9) var occlusion_sampler: sampler;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    // NOTE: w component of tangent shoudl specify LH RH coordinate system
    // always assume RH so ignore this value
    let T = normalize((model * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    let N = normalize((model * vec4<f32>(in.normal, 0.0)).xyz);
    let B = cross(N, T);
    let TBN = mat3x3f(T, B, N);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    // NOTE: since TBN rotates normal and no normal texture is used assume normal is (0,0,1)
    // need to move this step to fragment shader if using normal textures
    let surface_normal = vec3f(0.0, 0.0, 1.0);
    out.normal = normalize(TBN * surface_normal);
    out.tangent = in.tangent;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) tangent: vec4f,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4f(1.0, 1.0, 1.0, 1.0);
    // return vec4f(in.color, 1.0);
    // return vec4f(in.uv, 0.0, 1.0);

    return textureSample(normal_texture, normal_sampler, in.uv);
// return textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
// return textureSample(base_color_texture, base_color_sampler, in.uv);
// return textureSample(occlusion_texture, occlusion_sampler, in.uv);
}

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
