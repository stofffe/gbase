import "math"

fn custom_color() -> vec4f {
    if true {
        // return vec4f(0.0, 0.0, 1.0, 1.0);
    }

    let color1 = vec3f(1.0, 0.0, 0.0);
    let color2 = vec3f(0.0, 1.0, 0.0);
    let p = 1.0;

    return vec4f(lerp(color1, color2, p), 1.0);
}
