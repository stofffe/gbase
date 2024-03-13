use gbase::{filesystem, render, Callbacks, Context, ContextBuilder, LogLevel};
use std::sync::mpsc;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

const INPUT_SIZE: u32 = 8;
const INPUT_MEM_SIZE: u64 = std::mem::size_of::<u32>() as u64 * INPUT_SIZE as u64;
const OUTPUT_SIZE: u32 = 4;
const OUTPUT_MEM_SIZE: u64 = std::mem::size_of::<u32>() as u64 * OUTPUT_SIZE as u64;

struct App {
    input_buffer: render::RawBuffer,
    output_buffer: render::RawBuffer,
    cpu_buffer: render::RawBuffer,
    bindgroup: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        // Buffers
        let input_buffer = render::RawBufferBuilder::new()
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
            .build(ctx, INPUT_MEM_SIZE);
        let output_buffer = render::RawBufferBuilder::new()
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC)
            .build(ctx, OUTPUT_MEM_SIZE);
        let cpu_buffer = render::RawBufferBuilder::new()
            .usage(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST)
            .build(ctx, OUTPUT_MEM_SIZE);
        let (bindgroup_layout, bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                // input
                render::BindGroupCombinedEntry::new(input_buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::COMPUTE)
                    .storage(true),
                // output
                render::BindGroupCombinedEntry::new(output_buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::COMPUTE)
                    .storage(false),
            ])
            .build(ctx);

        let shader_str = filesystem::load_string(ctx, "compute.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new(&shader_str).build(ctx);

        let compute_pipeline = render::ComputePipelineBuilder::new(&shader)
            .bind_groups(&[&bindgroup_layout])
            .build(ctx);

        Self {
            compute_pipeline,
            bindgroup,
            input_buffer,
            output_buffer,
            cpu_buffer,
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
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
        compute_pass.set_bind_group(0, &self.bindgroup, &[]);
        compute_pass.dispatch_workgroups(OUTPUT_SIZE, 1, 1);
        drop(compute_pass);

        encoder.copy_buffer_to_buffer(
            self.output_buffer.buf(),
            0,
            self.cpu_buffer.buf(),
            0,
            OUTPUT_MEM_SIZE,
        );
        // submit here to be able to read in the same frame
        queue.submit(Some(encoder.finish()));

        // read data from output buffer
        let data: Vec<u32> = read_buffer_sync(device, self.cpu_buffer.buf());
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
