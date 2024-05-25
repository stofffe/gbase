// Vertex

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> FragmentInput {
    var out: FragmentInput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var position_tex: texture_2d<f32>;
@group(0) @binding(2) var albedo_tex: texture_2d<f32>;
@group(0) @binding(3) var normal_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<f32>;
@group(0) @binding(5) var<uniform> camera: Camera;
@group(0) @binding(6) var<uniform> light: vec3<f32>;
@group(0) @binding(7) var<uniform> debug_input: DebugInput;

struct DebugInput { btn1: u32, btn2: u32, btn3: u32, btn4: u32, btn5: u32, btn6: u32, btn7: u32, btn8: u32, btn9: u32 };
fn btn1_pressed() -> bool { return debug_input.btn1 == 1u; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u; }
fn btn7_pressed() -> bool { return debug_input.btn7 == 1u; }
fn btn8_pressed() -> bool { return debug_input.btn8 == 1u; }
fn btn9_pressed() -> bool { return debug_input.btn9 == 1u; }

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
};

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let position = textureSample(position_tex, samp, in.uv).xyz;
    let albedo = textureSample(albedo_tex, samp, in.uv).xyz;
    var normal = textureSample(normal_tex, samp, in.uv).xyz;
    normal = normalize(normal * 2.0 - 1.0); // [0,1] -> [-1,1]

    let ao = textureSample(roughness_tex, samp, in.uv).r;
    let roughness = textureSample(roughness_tex, samp, in.uv).g; // Invert so higher => more relfection
    let metalness = textureSample(roughness_tex, samp, in.uv).b; // 0 = no metal, 1 = full metal

    // Phong shading
    let light_dir = normalize(light - position);
    let view_dir = normalize(camera.position - position);
    let half_dir = normalize(light_dir + view_dir);

    let ambient = 0.05;
    let diffuse = 0.3 * saturate(dot(normal, light_dir)) * (1.0 - metalness);
    let specular = 0.7 * pow(saturate(dot(normal, half_dir)), 31.0);

    let light = saturate(ambient + diffuse + specular) * ao;
    //let light = saturate(specular) * ao;

    if btn1_pressed() {
        return vec4<f32>(albedo, 1.0);
    }
    if btn2_pressed() {
        return vec4<f32>(normal, 1.0);
    }
    if btn3_pressed() {
        return vec4<f32>(position, 1.0);
    }
    if btn4_pressed() {
        return vec4<f32>(ao, ao, ao, 1.0);
    }
    if btn5_pressed() {
        return vec4<f32>(roughness, roughness, roughness, 1.0);
    }
    if btn6_pressed() {
        return vec4<f32>(metalness, metalness, metalness, 1.0);
    }
    if btn7_pressed() {
        return vec4<f32>(diffuse, diffuse, diffuse, 1.0);
    }
    if btn8_pressed() {
        return vec4<f32>(specular, specular, specular, 1.0);
    }
    return vec4<f32>(albedo * light, 1.0);
}
