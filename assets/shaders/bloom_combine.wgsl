@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var bloom_texture: texture_2d<f32>;
@group(0) @binding(2) var out_texture: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(16, 16, 1)
fn extract(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dim = vec2<u32>(textureDimensions(in_texture));
    if (global_id.x >= dim.x || global_id.y >= dim.y) {
        return;
    }

    let in_color = textureLoad(in_texture, global_id.xy, 0);
    let bloom = textureLoad(bloom_texture, global_id.xy, 0);

    let color = in_color + bloom;

    textureStore(out_texture, global_id.xy, color);
}
