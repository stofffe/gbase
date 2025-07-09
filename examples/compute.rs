use gbase::{filesystem, render, Callbacks, Context};
use std::sync::mpsc;

fn main() {
    gbase::run_sync::<App>();
}

const INPUT_SIZE: u32 = 8;
const INPUT_MEM_SIZE: u64 = std::mem::size_of::<u32>() as u64 * INPUT_SIZE as u64;
const OUTPUT_SIZE: u32 = 4;
const OUTPUT_MEM_SIZE: u64 = std::mem::size_of::<u32>() as u64 * OUTPUT_SIZE as u64;

struct App {
    input_buffer: render::RawBuffer<u32>,
    output_buffer: render::RawBuffer<u32>,
    cpu_buffer: render::RawBuffer<u32>,
    bindgroup: render::ArcBindGroup,
    compute_pipeline: render::ArcComputePipeline,
}

impl Callbacks for App {
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        // Buffers
        let input_buffer = render::RawBufferBuilder::new(INPUT_MEM_SIZE)
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        let output_buffer = render::RawBufferBuilder::new(OUTPUT_MEM_SIZE)
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC)
            .build(ctx);
        let cpu_buffer = render::RawBufferBuilder::new(OUTPUT_MEM_SIZE)
            .usage(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // input
                render::BindGroupLayoutEntry::new()
                    .compute()
                    .storage_readonly(),
                // output
                render::BindGroupLayoutEntry::new().compute().storage(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // input
                render::BindGroupEntry::Buffer(input_buffer.buffer()),
                // output
                render::BindGroupEntry::Buffer(output_buffer.buffer()),
            ])
            .build(ctx);

        let shader_str = filesystem::load_s!("shaders/compute.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let compute_pipeline =
            render::ComputePipelineBuilder::new(shader, pipeline_layout).build(ctx);

        Self {
            compute_pipeline,
            bindgroup,
            input_buffer,
            output_buffer,
            cpu_buffer,
        }
    }
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> bool {
        let device = render::device(ctx);
        let queue = render::queue(ctx);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render encodeer"),
        });

        // write data to input
        let mut input = vec![0u32; INPUT_SIZE as usize];
        for i in 0..INPUT_SIZE {
            input[i as usize] = i;
        }
        self.input_buffer.write(ctx, &input);
        println!("INPUT {:?}", input);

        // run compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
        compute_pass.dispatch_workgroups(OUTPUT_SIZE, 1, 1);
        drop(compute_pass);

        encoder.copy_buffer_to_buffer(
            self.output_buffer.buffer_ref(),
            0,
            self.cpu_buffer.buffer_ref(),
            0,
            OUTPUT_MEM_SIZE,
        );
        // submit here to be able to read in the same frame
        queue.submit(Some(encoder.finish()));

        // read data from output buffer
        let data: Vec<u32> = read_buffer_sync(device, self.cpu_buffer.buffer_ref());
        println!("DATA {:?}", data);

        false
    }
}

/// DEBUG
///
/// Reads a mapped buffer
///
/// Panics if buffer is not mapped
fn read_buffer_sync<T: bytemuck::AnyBitPattern>(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
) -> Vec<T> {
    let buffer_slice = buffer.slice(..);
    let (sc, rc) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        sc.send(res).unwrap();
    });
    device.poll(wgpu::MaintainBase::Wait);
    let _ = rc.recv().unwrap();
    let data = buffer_slice.get_mapped_range();
    let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    buffer.unmap();
    result
}
