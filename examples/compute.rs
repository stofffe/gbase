use gbase::{
    render::{self},
    Callbacks, Context, ContextBuilder, LogLevel,
};
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
    // compute_pipeline: wgpu::ComputePipeline,
    compute_pipeline: render::ComputePipeline,
    input_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    cpu_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);

        // Shader
        let shader = render::ShaderBuilder::new("compute.wgsl".to_string())
            .build(ctx)
            .await;

        // Buffers
        let input_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("input buffer"),
            size: INPUT_MEM_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output buffer"),
            size: OUTPUT_MEM_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let cpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cpu buffer"),
            size: OUTPUT_MEM_SIZE,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind group layout"),
            entries: &[
                // input
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // output
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind group"),
            layout: &bind_group_layout,
            entries: &[
                // input
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                // output
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline = render::ComputePipelineBuilder::new(&shader)
            .bind_group_layouts(&[&bind_group_layout])
            .build(ctx);

        Self {
            compute_pipeline,
            bind_group,
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
        queue.write_buffer(&self.input_buffer, 0, bytemuck::cast_slice(&input));
        println!("INPUT {:?}", input);

        // run compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(self.compute_pipeline.pipeline());
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(OUTPUT_SIZE, 1, 1);
        drop(compute_pass);

        encoder.copy_buffer_to_buffer(&self.output_buffer, 0, &self.cpu_buffer, 0, OUTPUT_MEM_SIZE);
        queue.submit(Some(encoder.finish()));

        // read data from output buffer
        let data: Vec<u32> = read_buffer_sync(&device, &self.cpu_buffer);
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
