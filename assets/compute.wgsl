@group(0) @binding(0) var<storage, read> input: array<u32, 8>;
@group(0) @binding(1) var<storage, read_write> output: array<u32, 4>;

@compute
@workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    output[i] = input[i * u32(2)] + input[i * u32(2) + u32(1)];
}


