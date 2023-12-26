@group(0) @binding(0) var<storage, read> instances: array<GrassInstance>;
@group(0) @binding(1) var<storage, read_write> instance_count: atomic<u32>;

struct GrassInstance {  // align 16 size 
    pos: vec3<f32>,  // align 16 size 12
    rot: vec2<f32>   // 
};

@compute
@workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    atomicAdd(&instance_count, u32(1));
}
