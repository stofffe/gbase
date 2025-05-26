@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var out_texture: texture_storage_2d<rgba16float, write>;

const THRESHOLD = 1.0;

@compute @workgroup_size(16, 16, 1)
fn extract(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dim = vec2<u32>(textureDimensions(in_texture));
    if (global_id.x >= dim.x || global_id.y >= dim.y) {
        return;
    }

    let color = textureLoad(in_texture, global_id.xy, 0);

    var output_color = vec3f(0.0);
    let luminance = calculate_luminance(color.rgb);
    if luminance > THRESHOLD {
        output_color = color.rgb;
    }

    textureStore(out_texture, global_id.xy, vec4f(output_color, 1.0));
}

fn calculate_luminance(color: vec3f) -> f32 {
    return dot(color, vec3f(0.2126, 0.7152, 0.0722));
}
