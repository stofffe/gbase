@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(1, 1, 1)
fn horizontal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let color = textureLoad(in_texture, global_id.xy, 0);
    let tonemapped = tone_mapping_reinhard(color.xyz);
    textureStore(out_texture, global_id.xy, vec4f(tonemapped, 1.0));
}

fn tone_mapping_reinhard(color: vec3f) -> vec3f {
    return color / (color + vec3f(1.0));
}
