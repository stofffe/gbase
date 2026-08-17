fn add(a: f32, b: f32) -> f32 {
    return a + b;
}

fn lerp(a: vec3f, b: vec3f, p: f32) -> vec3f {
// return
// vec3f(0.0,0.0,0.0);
    return a * (1 - p) + b * p;
}
