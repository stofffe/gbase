@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba16float, write>;

const weight: array<f32, 5> = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

@compute @workgroup_size(16, 16, 1)
fn horizontal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    let dim = vec2<i32>(textureDimensions(in_texture));
    if (x >= dim.x || y >= dim.y) {
        return;
    }

    var result = weight[0] * textureLoad(in_texture, vec2i(x, y), 0);
    for (var i = 1; i < 5; i++) {
        result += weight[i] * textureLoad(in_texture, vec2i(min(x + i, dim.x - 1), y), 0);
        result += weight[i] * textureLoad(in_texture, vec2i(max(x - i, 0), y), 0);
    }

    textureStore(out_texture, global_id.xy, result);
}

@compute @workgroup_size(16, 16, 1)
fn vertical(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    let dim = vec2<i32>(textureDimensions(in_texture));
    if (x >= dim.x || y >= dim.y) {
        return;
    }

    var result = weight[0] * textureLoad(in_texture, vec2i(x, y), 0);
    for (var i = 1; i < 5; i++) {
        result += weight[i] * textureLoad(in_texture, vec2i(x, min(y + i, dim.y - 1)), 0); // -1 ?
        result += weight[i] * textureLoad(in_texture, vec2i(x, max(y - i, 0)), 0);
    }

    textureStore(out_texture, global_id.xy, result);
}
