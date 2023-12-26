//@group(0) @binding(0) var tiles: array<Tile>;
//
//struct Tile {
//    pos: vec2<f32>,   
//};

@group(0) @binding(1) var<storage, read> instances: array<GrassInstance>;
@group(0) @binding(2) var<storage, read> vertices: array<Vertex>;

struct GrassInstance { 
    pos: vec3<f32>, 
    rot: vec2<f32> 
};
struct Vertex { 
    pos: vec2<f32> 
};

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    vertices[0].pos = vec3<f32>(100.0, 0.0, 100.0);
}

