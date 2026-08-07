fn add(a: f32, b: f32) -> f32 {
    return a + b;
}

fn lerp(a: vec3f, b: vec3f, p: f32) -> vec3f {
    return a * (1 - p) + b * p;
}
