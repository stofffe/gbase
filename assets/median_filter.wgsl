@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> debug_input: DebugInput;

const KERNEL_SIZE: i32 = 1;
const CELLS_X: i32 = (KERNEL_SIZE * 2 + 1);
const CELLS_Y: i32 = (KERNEL_SIZE * 2 + 1);
const CELLS: i32 = CELLS_X * CELLS_Y;

fn luminance(color: vec4<f32>) -> f32 {
    return (0.2126 * color.r) + (0.7152 * color.g) + (0.0722 * color.b);
}

// Bubble sort based on luminance
fn sort_by_brightness(arr: ptr<function, array<vec4<f32>, CELLS>>) {
    var temp: vec4<f32>;

    for (var i: u32 = 0u; i < u32(CELLS); i = i + 1u) {
        for (var j: u32 = 0u; j < u32(CELLS) - 1u - i; j = j + 1u) {
            if luminance((*arr)[j]) > luminance((*arr)[j + 1u]) {
                temp = (*arr)[j];
                (*arr)[j] = (*arr)[j + 1u];
                (*arr)[j + 1u] = temp;
            }
        }
    }
}

fn get_index(x: i32, y: i32) -> u32 {
    return u32((y + (i32(CELLS_Y) / 2)) * i32(CELLS_X) + (x + (i32(CELLS_X) / 2)));
}
 
@compute
@workgroup_size(1,1,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let dim = vec2<i32>(textureDimensions(in_texture));

    var values: array<vec4<f32>, CELLS>;

    var sum = vec4<f32>(0.0);
    for (var i = -KERNEL_SIZE; i <= KERNEL_SIZE; i++) {
        for (var j = -KERNEL_SIZE; j <= KERNEL_SIZE; j++) {
            values[get_index(i, j)] = textureLoad(in_texture, vec2<i32>(x + i, y + j), 0);
        }
    }

    sort_by_brightness(&values);

    let middle = values[0];

    textureStore(out_texture, global_id.xy, middle);
}



    //sum /= f32(visited);

    //for (var i = max(x - KERNEL_SIZE, 0); i <= min(x + KERNEL_SIZE, dim.x - 1); i++) {
    //    for (var j = max(y - KERNEL_SIZE, 0); j <= min(y + KERNEL_SIZE, dim.y - 1); j++) {
    //        values[get_index(i, j)] += textureLoad(in_texture, vec2<i32>(i, j), 0);
    //    }
    //}

    //if x < 1 || y < 1 || x > dim.x - 2 || y > dim.y - 2 {
    //    var pixel = textureLoad(in_texture, global_id.xy, 0);
    //    textureStore(out_texture, global_id.xy, pixel);
    //    return;
    //}

    //var local_array: array<vec4<f32>, 100> = array<vec4<f32>, 100>(
    //    textureLoad(in_texture, vec2<u32>(x - 1, y - 1), 0),
    //    textureLoad(in_texture, vec2<u32>(x, y - 1), 0),
    //    textureLoad(in_texture, vec2<u32>(x + 1, y - 1), 0),
    //    textureLoad(in_texture, vec2<u32>(x - 1, y), 0),
    //    textureLoad(in_texture, vec2<u32>(x, y), 0),
    //    textureLoad(in_texture, vec2<u32>(x + 1, y), 0),
    //    textureLoad(in_texture, vec2<u32>(x - 1, y + 1), 0),
    //    textureLoad(in_texture, vec2<u32>(x, y + 1), 0),
    //    textureLoad(in_texture, vec2<u32>(x + 1, y + 1), 0)
    //);

    //sort_by_brightness(&values);




//fn luminance(color: vec4<f32>) -> f32 {
//    return (0.2126 * color.r) + (0.7152 * color.g) + (0.0722 * color.b);
//}
//
//// Up to 10x10
//fn sort_by_brightness(arr: ptr<function, array<vec4<f32>, 100>>, size: u32) {
//    var temp: vec4<f32>;
//
//    // Bubble sort based on luminance, using dynamic size
//    for (var i: u32 = 0u; i < size; i = i + 1u) {
//        for (var j: u32 = 0u; j < size - 1u - i; j = j + 1u) {
//            if luminance((*arr)[j]) > luminance((*arr)[j + 1u]) {
//                temp = (*arr)[j];
//                (*arr)[j] = (*arr)[j + 1u];
//                (*arr)[j + 1u] = temp;
//            }
//        }
//    }
//}
//fn get_index(x: i32, y: i32) -> u32 {
//    return u32((y + 1) * 3 + (x + 1));
//}












struct DebugInput { btn1: u32, btn2: u32, btn3: u32, btn4: u32, btn5: u32, btn6: u32, btn7: u32, btn8: u32, btn9: u32};
fn btn1_pressed() -> bool { return debug_input.btn1 == 1u; }
fn btn2_pressed() -> bool { return debug_input.btn2 == 1u; }
fn btn3_pressed() -> bool { return debug_input.btn3 == 1u; }
fn btn4_pressed() -> bool { return debug_input.btn4 == 1u; }
fn btn5_pressed() -> bool { return debug_input.btn5 == 1u; }
fn btn6_pressed() -> bool { return debug_input.btn6 == 1u; }
fn btn7_pressed() -> bool { return debug_input.btn7 == 1u; }
fn btn8_pressed() -> bool { return debug_input.btn8 == 1u; }
fn btn9_pressed() -> bool { return debug_input.btn9 == 1u; }

