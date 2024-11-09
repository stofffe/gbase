@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;

const GAMMA = 1.0 / 2.2;

@compute
@workgroup_size(1,1,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let color = textureLoad(in_texture, global_id.xy, 0).xyz;
    let corrected_color = pow(color, vec3<f32>(GAMMA));

    textureStore(out_texture, global_id.xy, vec4<f32>(corrected_color, 1.0));
}

