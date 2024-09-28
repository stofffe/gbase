@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> debug_input: DebugInput;
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

@compute
@workgroup_size(1,1,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let dim = textureDimensions(in_texture);

    if x < 1 || y < 1 || x > dim.x - 2 || y > dim.y - 2 {
        var pixel = textureLoad(in_texture, global_id.xy, 0);
        textureStore(out_texture, global_id.xy, pixel);
        return;
    }

    if !btn1_pressed() {
        var pixel = textureLoad(in_texture, global_id.xy, 0);
        textureStore(out_texture, global_id.xy, pixel);
        return;
    }

    var sum = vec4<f32>(0.0);
    sum += textureLoad(in_texture, vec2<u32>(x - 1, y - 1), 0);
    sum += textureLoad(in_texture, vec2<u32>(x, y - 1), 0);
    sum += textureLoad(in_texture, vec2<u32>(x + 1, y - 1), 0);
    sum += textureLoad(in_texture, vec2<u32>(x - 1, y), 0);
    sum += textureLoad(in_texture, vec2<u32>(x, y), 0);
    sum += textureLoad(in_texture, vec2<u32>(x + 1, y), 0);
    sum += textureLoad(in_texture, vec2<u32>(x - 1, y + 1), 0);
    sum += textureLoad(in_texture, vec2<u32>(x, y + 1), 0);
    sum += textureLoad(in_texture, vec2<u32>(x + 1, y + 1), 0);
    sum /= vec4<f32>(9.0);

    textureStore(out_texture, global_id.xy, sum);
}
