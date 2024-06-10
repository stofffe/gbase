
// Vertex shader

@group(0) @binding(0) var normal_tex: texture_2d<f32>;
@group(0) @binding(1) var albedo_tex: texture_2d<f32>;
@group(0) @binding(2) var roughness_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> transform: mat4x4<f32>;
@group(0) @binding(5) var<uniform> camera: Camera;
@group(0) @binding(6) var<uniform> debug_input: DebugInput;

struct DebugInput { btn1: u32, btn2: u32, btn3: u32, btn4: u32, btn5: u32, btn6: u32, btn7: u32, btn8: u32, btn9: u32 };
fn btn1_pressed() -> bool { return debug_input.btn1 == 1u; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u; }

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) tangent: vec4<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let T = normalize((transform * vec4<f32>(in.tangent.xyz, 0.0)).xyz);
    let N = normalize((transform * vec4<f32>(in.normal, 0.0)).xyz);
    let B = cross(N, T);

    let world_position = transform * vec4<f32>(in.position, 1.0);
    var out: VertexOutput;
    //out.clip_position = camera.view_proj * transform * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.position = world_position.xyz;
    out.color = in.color;
    out.uv = in.uv;
    out.N = N;
    out.T = T;
    out.B = B;

    //out.normal = in.normal * in.tangent.w;
    out.normal = in.normal;
    return out;
}

// Fragment shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) T: vec3<f32>,
    @location(4) N: vec3<f32>,
    @location(5) B: vec3<f32>,

    @location(6) normal: vec3<f32>,
};

struct FragmentOutput {
    @location(0) position: vec4<f32>,
    @location(1) albedo: vec4<f32>,
    @location(2) normal: vec4<f32>,
    @location(3) roughness: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let albedo_tex = textureSample(albedo_tex, samp, in.uv);
    let roughness_tex = textureSample(roughness_tex, samp, in.uv);
    let normal_tex = textureSample(normal_tex, samp, in.uv);

    let TBN = mat3x3<f32>(in.T, in.B, in.N);
    var normal = normal_tex.xyz;
    normal = normal * 2.0 - 1.0; // [0,1] -> [-1,1]
    normal = normalize(TBN * normal); // transform on [-1,1]
    normal = (normal + 1.0) / 2.0; // [-1,1] -> [0,1]

    //let metalness = roughness_tex.b;
    //let roughness = roughness_tex.g;

    var out: FragmentOutput;
    out.position = vec4<f32>(in.position, 1.0);
    out.albedo = albedo_tex;
    out.normal = vec4<f32>(normal, 1.0);
    out.roughness = roughness_tex;

    return out;
}

//if btn1_pressed() {
//    normal = in.normal;
//}

//let light_dir = normalize(light - in.position);
//let view_dir = normalize(camera.position - in.position);
//let half_dir = normalize(light_dir + view_dir);

//let ambient = 0.01;
//let diffuse = 0.5 * saturate(dot(normal, light_dir));
//let specular = 1.0 * pow(saturate(dot(normal, half_dir)), 151.0);

//let light = ambient + diffuse + specular;
//let albedo_tex = textureSample(albedo_tex, samp, in.uv);
//let roughness_tex = textureSample(roughness_tex, samp, in.uv);
//let ambient_occ = roughness_tex.r;


//out.albedo = albedo * vec4<f32>(light, light, light, 1.0);
//return vec4<f32>(light, light, light, 1.0);
//return vec4<f32>(normal, 1.0);
//return vec4<f32>(in.uv, 0.0, 1.0);
//return vec4<f32>(1.0, 1.0, 1.0, 1.0);
//return vec4<f32>(light, light, light, 1.0);
//out.roughness = vec4<f32>(metalness, metalness, metalness, 1.0);
//out.roughness = vec4<f32>(ambient_occ, ambient_occ, ambient_occ, 1.0);
    //return vec4<f32>(in.tangent.xyz, 1.0);
