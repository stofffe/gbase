@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    kernel_size: i32,
};

fn luminance(color: vec4<f32>) -> f32 {
    return dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
}

// Bubble sort based on luminance
// TODO lower 100 to 49?
fn sort(arr: ptr<function, array<vec4<f32>, 100>>, length: u32) {
    var temp: vec4<f32>;

    for (var i: u32 = 0u; i < length; i = i + 1u) {
        for (var j: u32 = 0u; j < length - 1u - i; j = j + 1u) {
            if luminance((*arr)[j]) > luminance((*arr)[j + 1u]) {
                temp = (*arr)[j];
                (*arr)[j] = (*arr)[j + 1u];
                (*arr)[j + 1u] = temp;
            }
        }
    }
}

fn get_index(x: i32, y: i32, cells_x: i32, cells_y: i32) -> u32 {
    return u32((y + (i32(cells_y) / 2)) * i32(cells_x) + (x + (i32(cells_x) / 2)));
}
 
@compute
@workgroup_size(1,1,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let dim = vec2<i32>(textureDimensions(in_texture));

    let ksize = params.kernel_size;
    let cells_x = ksize * 2 + 1;
    let cells_y = ksize * 2 + 1;
    let cells = cells_x * cells_y;

    var values: array<vec4<f32>, 100>;

    var sum = vec4<f32>(0.0);
    for (var i = -ksize; i <= ksize; i++) {
        for (var j = -ksize; j <= ksize; j++) {
            values[get_index(i, j, cells_x, cells_y)] = textureLoad(in_texture, vec2<i32>(x + i, y + j), 0);
        }
    }

    sort(&values, u32(cells));

    let middle = values[0];

    textureStore(out_texture, global_id.xy, middle);
}
