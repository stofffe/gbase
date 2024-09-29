@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    kernel_size: i32,
};

@compute
@workgroup_size(1,1,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let dim = vec2<i32>(textureDimensions(in_texture));

    let ksize = params.kernel_size;
    let cells = (ksize * 2 + 1) * (ksize * 2 + 1);

    var sum = vec4<f32>(0.0);
    for (var i = -ksize; i <= ksize; i++) {
        for (var j = -ksize; j <= ksize; j++) {
            sum += textureLoad(in_texture, vec2<i32>(x + i, y + j), 0);
        }
    }
    sum /= f32(cells);

    textureStore(out_texture, global_id.xy, sum);
}
