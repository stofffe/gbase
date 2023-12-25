@group(0) @binding(0) var<storage, read_write> draw_args: DrawArgs;

struct DrawArgs {
    vertex_count: u32,
    instance_count: u32,
    base_vertex: u32,
    base_instance: u32,
};

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    draw_args.base_vertex = u32(0);
    draw_args.vertex_count = u32(11);
    draw_args.base_instance = u32(0);
    draw_args.instance_count = u32(100 * 100);
}



