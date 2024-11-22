@group(0) @binding(0) var output :  texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = global_id.xy;
    textureStore(output, coords, vec4(1.0));
}
