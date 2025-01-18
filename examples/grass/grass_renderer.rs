use encase::ShaderType;
use gbase::glam;
use gbase::wgpu;
use gbase::{
    filesystem,
    render::{self, ArcBindGroup, ArcComputePipeline, ArcRenderPipeline, CameraUniform},
    Context,
};
use glam::{vec2, Vec2, Vec3Swizzles};
use std::{mem::size_of, ops::Div};

const TILE_SIZE: f32 = 128.0;
const TILES_PER_SIDE: i32 = 3;
const BLADES_PER_SIDE: u32 = 16 * 30; // must be > 16 due to dispatch(B/16, B/16, 1) workgroups(16,16,1)
const BLADES_PER_TILE: u32 = BLADES_PER_SIDE * BLADES_PER_SIDE;

pub struct GrassRenderer {
    instances: [render::RawBuffer<GrassInstanceGPU>; 2],
    indirect_buffer: [render::RawBuffer<DrawIndirectArgs>; 2],
    instance_count: render::RawBuffer<u32>,
    tile_buffer: render::UniformBuffer<Tile>,
    app_info: render::AppInfo,

    instance_pipeline: ArcComputePipeline,
    instance_bindgroup: [ArcBindGroup; 2],

    draw_pipeline: ArcComputePipeline,
    draw_bindgroup: [ArcBindGroup; 2],

    render_pipeline: ArcRenderPipeline,
    render_bindgroup: ArcBindGroup,

    debug_input: render::DebugInput,
}

impl GrassRenderer {
    pub fn render(
        &mut self,
        ctx: &Context,
        camera: &render::PerspectiveCamera,
        deferred_buffers: &render::DeferredBuffers,
    ) {
        let queue = render::queue(ctx);
        self.app_info.update_buffer(ctx);
        self.debug_input.update_buffer(ctx);

        let curr_tile = camera.pos.xz().div(TILE_SIZE).floor() * TILE_SIZE;

        let lower = -TILES_PER_SIDE / 2;
        let upper = TILES_PER_SIDE / 2;

        let mut tiles = Vec::new();
        for y in lower..=upper {
            for x in lower..=upper {
                let tile = curr_tile + vec2(x as f32, y as f32) * TILE_SIZE;
                tiles.push(tile);
            }
        }

        for (i, tile) in tiles.into_iter().enumerate() {
            let mut encoder = render::EncoderBuilder::new().build(ctx);
            // Alternate buffers to allow for GPU pipelining
            let i_cur = i % 2;

            //
            // Compute
            //
            self.instance_count.write(ctx, &[0u32]);
            self.tile_buffer.write(
                ctx,
                &Tile {
                    pos: tile,
                    size: TILE_SIZE,
                    blades_per_side: BLADES_PER_SIDE as f32,
                },
            );
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute pass"),
                timestamp_writes: None,
            });

            // instance
            compute_pass.set_pipeline(&self.instance_pipeline);
            compute_pass.set_bind_group(0, Some(self.instance_bindgroup[i_cur].as_ref()), &[]);
            compute_pass.dispatch_workgroups(BLADES_PER_SIDE / 16, BLADES_PER_SIDE / 16, 1);

            // draw
            compute_pass.set_pipeline(&self.draw_pipeline);
            compute_pass.set_bind_group(0, Some(self.draw_bindgroup[i_cur].as_ref()), &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);

            drop(compute_pass);

            //
            // Render
            //
            let attachments = &deferred_buffers.color_attachments();
            let mut render_pass = render::RenderPassBuilder::new()
                .color_attachments(attachments)
                .depth_stencil_attachment(deferred_buffers.depth_stencil_attachment_load())
                .build(&mut encoder);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.instances[i_cur].slice(..));
            render_pass.set_bind_group(0, Some(self.render_bindgroup.as_ref()), &[]);
            render_pass.draw_indirect(self.indirect_buffer[i_cur].buffer_ref(), 0);

            drop(render_pass);

            queue.submit(Some(encoder.finish()));
        }
    }

    pub fn new(
        ctx: &mut Context,
        deferred_buffers: &render::DeferredBuffers,
        camera_buffer: &render::UniformBuffer<CameraUniform>,
    ) -> Self {
        let instances = [
            render::RawBufferBuilder::new(render::RawBufferSource::Size(
                GrassInstanceGPU::SIZE * BLADES_PER_TILE as u64,
            ))
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
            .build(ctx),
            render::RawBufferBuilder::new(render::RawBufferSource::Size(
                GrassInstanceGPU::SIZE * BLADES_PER_TILE as u64,
            ))
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
            .build(ctx),
        ];

        let instance_count =
            render::RawBufferBuilder::new(render::RawBufferSource::Size(size_of::<u32>() as u64))
                .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
                .build(ctx);

        #[rustfmt::skip]
        let indirect_buffer = [
            render::RawBufferBuilder::new(render::RawBufferSource::Size(size_of::<wgpu::util::DrawIndirectArgs>() as u64))
                .usage( wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,)
                .build(ctx),
            render::RawBufferBuilder::new(render::RawBufferSource::Size(size_of::<wgpu::util::DrawIndirectArgs>() as u64))
                .usage( wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,)
                .build(ctx),
        ];

        let perlin_noise_bytes = filesystem::load_b!("textures/perlin_noise.png").unwrap();
        let perlin_noise_texture =
            render::TextureBuilder::new(render::TextureSource::Bytes(perlin_noise_bytes))
                .build(ctx)
                .with_default_view(ctx);
        let perlin_noise_sampler = render::SamplerBuilder::new().build(ctx);

        let tile_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let app_info = render::AppInfo::new(ctx);
        let debug_input = render::DebugInput::new(ctx);

        // Instance
        let instance_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // instances
                render::BindGroupLayoutEntry::new().storage().compute(),
                // instance count
                render::BindGroupLayoutEntry::new().storage().compute(),
                // tile
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // perlin texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // perlin texture sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .compute(),
                // camera
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // app info
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // debug input
                render::BindGroupLayoutEntry::new().uniform().compute(),
            ])
            .build(ctx);

        let instance_bindgroup = [
            render::BindGroupBuilder::new(instance_bindgroup_layout.clone())
                .entries(vec![
                    // instances
                    render::BindGroupEntry::Buffer(instances[0].buffer()),
                    // instance count
                    render::BindGroupEntry::Buffer(instance_count.buffer()),
                    // tile
                    render::BindGroupEntry::Buffer(tile_buffer.buffer()),
                    // perlin texture
                    render::BindGroupEntry::Texture(perlin_noise_texture.view()),
                    // perlin texture sampler
                    render::BindGroupEntry::Sampler(perlin_noise_sampler.clone()),
                    // camera
                    render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                    // app info
                    render::BindGroupEntry::Buffer(app_info.buffer()),
                    // debug input
                    render::BindGroupEntry::Buffer(debug_input.buffer()),
                ])
                .build(ctx),
            render::BindGroupBuilder::new(instance_bindgroup_layout.clone())
                .entries(vec![
                    // instances
                    render::BindGroupEntry::Buffer(instances[1].buffer()),
                    // instance count
                    render::BindGroupEntry::Buffer(instance_count.buffer()),
                    // tile
                    render::BindGroupEntry::Buffer(tile_buffer.buffer()),
                    // perlin texture
                    render::BindGroupEntry::Texture(perlin_noise_texture.view()),
                    // perlin texture sampler
                    render::BindGroupEntry::Sampler(perlin_noise_sampler),
                    // camera
                    render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                    // app info
                    render::BindGroupEntry::Buffer(app_info.buffer()),
                    // debug input
                    render::BindGroupEntry::Buffer(debug_input.buffer()),
                ])
                .build(ctx),
        ];

        let instance_shader_str =
            filesystem::load_s!("shaders/grass_compute_instance.wgsl").unwrap();
        let instance_shader = render::ShaderBuilder::new(instance_shader_str).build(ctx);

        let instance_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![instance_bindgroup_layout])
            .build(ctx);
        let instance_pipeline =
            render::ComputePipelineBuilder::new(instance_shader, instance_pipeline_layout)
                .label("instance".to_string())
                .build(ctx);

        // Draw
        let draw_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // indirect buffer
                render::BindGroupLayoutEntry::new().storage().compute(),
                // instance count
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .compute(),
            ])
            .build(ctx);
        let draw_bindgroup = [
            render::BindGroupBuilder::new(draw_bindgroup_layout.clone())
                .entries(vec![
                    // indirect buffer
                    render::BindGroupEntry::Buffer(indirect_buffer[0].buffer()),
                    // instance count
                    render::BindGroupEntry::Buffer(instance_count.buffer()),
                ])
                .build(ctx),
            render::BindGroupBuilder::new(draw_bindgroup_layout.clone())
                .entries(vec![
                    // indirect buffer
                    render::BindGroupEntry::Buffer(indirect_buffer[1].buffer()),
                    // instance count
                    render::BindGroupEntry::Buffer(instance_count.buffer()),
                ])
                .build(ctx),
        ];

        let draw_shader_str = filesystem::load_s!("shaders/grass_compute_draw.wgsl").unwrap();
        let draw_compute_shader = render::ShaderBuilder::new(draw_shader_str).build(ctx);

        let draw_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![draw_bindgroup_layout])
            .build(ctx);
        let draw_pipeline =
            render::ComputePipelineBuilder::new(draw_compute_shader, draw_pipeline_layout.clone())
                .label("draw".to_string())
                .build(ctx);

        // Render
        let render_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // Camera
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // Debug input
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // app info
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
            ])
            .build(ctx);
        let render_bindgroup = render::BindGroupBuilder::new(render_bindgroup_layout.clone())
            .entries(vec![
                // Camera
                render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                // Debug
                render::BindGroupEntry::Buffer(debug_input.buffer()),
                // App info
                render::BindGroupEntry::Buffer(app_info.buffer()),
            ])
            .build(ctx);

        let render_shader_str = filesystem::load_s!("shaders/grass.wgsl").unwrap();
        let render_shader = render::ShaderBuilder::new(render_shader_str).build(ctx);
        let render_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![render_bindgroup_layout])
            .build(ctx);
        let render_pipeline =
            render::RenderPipelineBuilder::new(render_shader, render_pipeline_layout)
                .label("render".to_string())
                .buffers(vec![GrassInstanceGPU::desc()])
                .multiple_targets(deferred_buffers.targets().to_vec())
                .depth_stencil(deferred_buffers.depth_stencil_state())
                .topology(wgpu::PrimitiveTopology::TriangleStrip)
                // .polygon_mode(wgpu::PolygonMode::Line)
                .build(ctx);

        Self {
            instances,
            instance_count,
            indirect_buffer,
            tile_buffer,
            app_info,
            instance_pipeline,
            instance_bindgroup,
            draw_pipeline,
            draw_bindgroup,
            render_pipeline,
            render_bindgroup,

            debug_input,
        }
    }
}

#[derive(ShaderType)]
struct Tile {
    pos: Vec2,
    size: f32,
    blades_per_side: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct GrassInstanceGPU {
    position: [f32; 3],
    hash: [u32; 1],
    facing: [f32; 2],
    wind: [f32; 1],
    pad: [f32; 1],
    height: [f32; 1],
    tilt: [f32; 1],
    bend: [f32; 1],
    width: [f32; 1],
}

impl GrassInstanceGPU {
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    const ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
        1=>Float32x3,   // pos
        2=>Uint32,      // hash
        3=>Float32x2,   // facing
        4=>Float32,     // wind
        5=>Float32,     // pad
        6=>Float32,     // height
        7=>Float32,     // tilt
        8=>Float32,     // bend
        9=>Float32,     // width
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

// assert DrawIndirectArgs == wgpu::DrawIndirectArgs
const _: [(); 1] = [(); (std::mem::size_of::<DrawIndirectArgs>()
    == std::mem::size_of::<wgpu::util::DrawIndirectArgs>()) as usize]; // Passes

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}
