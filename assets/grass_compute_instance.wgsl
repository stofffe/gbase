@group(0) @binding(0) var<storage, read_write> instances: array<GrassInstance>;
@group(0) @binding(1) var<storage, read_write> instance_count: atomic<u32>;

struct GrassInstance {  // align 16 size 
    pos: vec3<f32>,
    hash: u32,
    facing: vec2<f32>,
    wind: f32,
    pad: f32,
};

const blades_per_side = 16.0 * 1.0;
const tile_size = 5.0;

@compute
@workgroup_size(16,16,1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let z = global_id.y;
    if true {
        let i = atomicAdd(&instance_count, 1u);

        instances[i].pos = vec3<f32>(
            (f32(x) / blades_per_side) * tile_size,
            0.0,
            (f32(z) / blades_per_side) * tile_size,
        );
        instances[i].hash = hash(x + z); // TODO probably uniform
        instances[i].wind = 0.5;
        instances[i].facing = vec2<f32>(1.0, 2.0); // x z
    }
}

fn hash(input: u32) -> u32 {
    let state = input * u32(747796405) + u32(2891336453u);
    let word = ((state >> ((state >> u32(28)) + u32(4))) ^ state) * u32(277803737);
    return (word >> u32(22)) ^ word;
}
