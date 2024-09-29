@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> debug_input: DebugInput;

const KERNEL_SIZE: i32 = 1;
const CELLS_X: i32 = (KERNEL_SIZE * 2 + 1);
const CELLS_Y: i32 = (KERNEL_SIZE * 2 + 1);
const CELLS: i32 = CELLS_X * CELLS_Y;

@compute
@workgroup_size(1,1,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let dim = vec2<i32>(textureDimensions(in_texture));

    var sum = vec4<f32>(0.0);
    for (var i = -KERNEL_SIZE; i <= KERNEL_SIZE; i++) {
        for (var j = -KERNEL_SIZE; j <= KERNEL_SIZE; j++) {
            sum += textureLoad(in_texture, vec2<i32>(x + i, y + j), 0);
        }
    }
    sum /= f32(CELLS);

    textureStore(out_texture, global_id.xy, sum);
}

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
