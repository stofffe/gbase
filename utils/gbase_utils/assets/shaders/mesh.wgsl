struct VertexInput {
    @builtin(instance_index) index: u32,
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tangent: vec4f,
    @location(3) uv: vec2f,
    @location(4) color: vec3f,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> lights: PbrLights;
@group(0) @binding(2) var<storage, read> instances: array<Instance>;
@group(0) @binding(3) var base_color_texture: texture_2d<f32>;
@group(0) @binding(4) var base_color_sampler: sampler;
@group(0) @binding(5) var normal_texture: texture_2d<f32>;
@group(0) @binding(6) var normal_sampler: sampler;
@group(0) @binding(7) var metallic_roughness_texture: texture_2d<f32>;
@group(0) @binding(8) var metallic_roughness_sampler: sampler;
@group(0) @binding(9) var occlusion_texture: texture_2d<f32>;
@group(0) @binding(10) var occlusion_sampler: sampler;

// NOTE: alignment
struct Instance {
    // transform
    transform: mat4x4f,

    // material
    color_factor: vec4f,
    roughness_factor: f32,
    metallic_factor: f32,
    occlusion_strength: f32,
    normal_scale: f32,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let model = instances[in.index].transform;
    // NOTE: w component of tangent shoudl specify LH RH coordinate system
    // always assume RH so ignore this value
    let T = normalize((model * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    let N = normalize((model * vec4<f32>(in.normal, 0.0)).xyz);
    let B = cross(N, T);

    var out: VertexOutput;
    let position = model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * position;
    out.uv = in.uv;
    out.color = in.color;
    // NOTE: since TBN rotates normal and no normal texture is used assume normal is (0,0,1)
    // need to move this step to fragment shader if using normal textures
    out.pos = position.xyz;
    out.T = T;
    out.B = B;
    out.N = N;
    out.index = in.index;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3f,
    @location(1) color: vec3f,
    @location(2) uv: vec2f,
    @location(5) T: vec3f,
    @location(6) B: vec3f,
    @location(7) N: vec3f,
    @location(8) index: u32,

}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal_tex = textureSample(normal_texture, normal_sampler, in.uv);
    let base_color_tex = textureSample(base_color_texture, base_color_sampler, in.uv);
    let roughness_tex = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
    let occlusion_tex = textureSample(occlusion_texture, occlusion_sampler, in.uv);

    let roughness = roughness_tex.g;
    let metallicness = roughness_tex.b;
    let occlusion = occlusion_tex.r;

    let TBN = mat3x3f(in.T, in.B, in.N);
    let unpacked_normal = normalize(normal_tex.xyz * 2.0 - 1.0); // [0,1] -> [-1,1]
    let normal = normalize(TBN * unpacked_normal);

    let light_dir = normalize(-lights.main_light_dir);
    let view_dir = normalize(camera.pos - in.pos);
    let half_dir = normalize(light_dir + view_dir);

    let ambient = 0.01;
    let diffuse = 0.5 * saturate(dot(normal, light_dir));
    let specular_factor = 1.0 / (roughness * roughness);
    let specular = 1.0 * pow(saturate(dot(normal, half_dir)), specular_factor);
    let light = ambient + diffuse + specular;

    // if true {
    //     let col = transforms[in.index].color;
    //     return vec4f(col, 1.0);
    // }
    // return
    // if in.index == 0 {
    //     return vec4f(1.0, 0.0, 0.0, 1.0);
    // } else if in.index == 1 {
    //     return vec4f(0.0, 1.0, 0.0, 1.0);
    // } else if in.index == 2 {
    //     return vec4f(0.0, 0.0, 1.0, 1.0);
    // }

    if true {
    // return vec4f(1.0, 1.0, 1.0, 1.0);
    // return vec4f(normal, 1.0);
    // return vec4f(roughness, roughness, roughness, 1.0);
    }

    // let color = base_color_tex.xyz * material.base_color_factor.xyz * light;
    let color = base_color_tex.xyz * instances[in.index].color_factor.xyz * light;
    return vec4f(color, 1.0);
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

struct PbrMaterial {
    base_color_factor: vec4f,
    roughness_factor: f32,
    metallic_factor: f32,
    occlusion_strength: f32,
    normal_scale: f32,
}

struct PbrLights {
    main_light_dir: vec3f,
}
