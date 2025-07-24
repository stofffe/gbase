use encase::ShaderType;
use gbase::{
    asset::{self, AssetHandle, ShaderLoader},
    filesystem,
    glam::{vec2, Vec2, Vec3Swizzles},
    input,
    render::{
        self, ArcBindGroupLayout, ArcPipelineLayout, ColorTargetState, GpuImage,
        RenderPassColorAttachment,
    },
    wgpu, Context,
};
use gbase_utils::{CameraFrustum, DeferredBuffers};
use std::{mem::size_of, ops::Div};

const TILE_SIZE: f32 = 128.0;
const TILES_PER_SIDE: i32 = 3;
const BLADES_PER_SIDE: u32 = 16 * 30; // must be > 16 due to dispatch(B/16, B/16, 1) workgroups(16,16,1)
const BLADES_PER_TILE: u32 = BLADES_PER_SIDE * BLADES_PER_SIDE;

pub enum RenderMode<'a> {
    Forward {
        view: &'a wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        depth_buffer: &'a render::DepthBuffer,
    },
    Deferred {
        buffers: &'a DeferredBuffers,
    },
}

pub struct GrassRenderer {
    instances: [render::RawBuffer<GrassInstanceGPU>; 2],
    indirect_buffer: [render::RawBuffer<DrawIndirectArgs>; 2],
    instance_count: [render::RawBuffer<u32>; 2],
    tile_buffer: [render::UniformBuffer<Tile>; 2],

    instance_pipeline_layout: ArcPipelineLayout,
    instance_bindgroup_layout: ArcBindGroupLayout,
    instance_shader_handle: AssetHandle<render::ShaderBuilder>,

    draw_pipeline_layout: ArcPipelineLayout,
    draw_bindgroup_layout: ArcBindGroupLayout,
    draw_shader_handle: AssetHandle<render::ShaderBuilder>,

    render_pipeline_layout: ArcPipelineLayout,
    render_bindgroup_layout: ArcBindGroupLayout,
    render_deferred_shader_handle: AssetHandle<render::ShaderBuilder>,
    render_forward_shader_handle: AssetHandle<render::ShaderBuilder>,

    app_info: gbase_utils::AppInfo,
    debug_input: gbase_utils::DebugInput,

    perlin_noise_texture: GpuImage,
}

impl GrassRenderer {
    pub fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
        let instances = [
            render::RawBufferBuilder::new(GrassInstanceGPU::SIZE * BLADES_PER_TILE as u64)
                .label("instance buffer 1")
                .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
                .build(ctx),
            render::RawBufferBuilder::new(GrassInstanceGPU::SIZE * BLADES_PER_TILE as u64)
                .label("instance buffer 2")
                .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
                .build(ctx),
        ];

        let instance_count = [
            render::RawBufferBuilder::new(size_of::<u32>() as u64)
                .label("instance count 1")
                .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
                .build(ctx),
            render::RawBufferBuilder::new(size_of::<u32>() as u64)
                .label("instance count 2")
                .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
                .build(ctx),
        ];

        #[rustfmt::skip]
        let indirect_buffer = [
            render::RawBufferBuilder::new(size_of::<wgpu::util::DrawIndirectArgs>() as u64)
                .label("indirect buffer 1")
                .usage( wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,)
                .build(ctx),
            render::RawBufferBuilder::new(size_of::<wgpu::util::DrawIndirectArgs>() as u64)
                .label("indirect buffer 2")
                .usage( wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,)
                .build(ctx),
        ];

        let perlin_noise_texture = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/perlin_noise.png").unwrap(),
        )
        .unwrap()
        .label("perlin noise")
        .build(ctx)
        .with_default_sampler_and_view(ctx);

        let tile_buffer = [
            render::UniformBufferBuilder::new()
                .label("tiles 1")
                .build(ctx),
            render::UniformBufferBuilder::new()
                .label("tiles 2")
                .build(ctx),
        ];
        let app_info = gbase_utils::AppInfo::new(ctx);
        let debug_input = gbase_utils::DebugInput::new(ctx);

        // Instance
        let instance_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .label("instance")
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
                // camera frustum
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // app info
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // debug input
                render::BindGroupLayoutEntry::new().uniform().compute(),
            ])
            .build(ctx);

        let instance_shader_handle = asset::AssetBuilder::load(
            "assets/shaders/grass_compute_instance.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);

        let instance_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![instance_bindgroup_layout.clone()])
            .build(ctx);

        //
        // Draw
        //

        // Draw with indirect draw calls
        let draw_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .label("draw")
            .entries(vec![
                // indirect buffer
                render::BindGroupLayoutEntry::new().storage().compute(),
                // instance count
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .compute(),
            ])
            .build(ctx);

        let draw_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            "assets/shaders/grass_compute_draw.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);

        let draw_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![draw_bindgroup_layout.clone()])
            .build(ctx);

        // Render
        let render_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .label("render")
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

        let render_deferred_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            "assets/shaders/grass_deferred.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);
        let render_forward_shader_handle =
            asset::AssetBuilder::load::<ShaderLoader>("assets/shaders/grass.wgsl", ShaderLoader {})
                .watch(cache)
                .build(cache);
        let render_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![render_bindgroup_layout.clone()])
            .build(ctx);

        Self {
            instances,
            instance_count,
            indirect_buffer,
            tile_buffer,

            instance_pipeline_layout,
            instance_bindgroup_layout,
            instance_shader_handle,

            draw_pipeline_layout,
            draw_bindgroup_layout,
            draw_shader_handle,

            render_pipeline_layout,
            render_deferred_shader_handle,
            render_forward_shader_handle,
            render_bindgroup_layout,

            debug_input,
            app_info,

            perlin_noise_texture,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        camera: &gbase_utils::Camera,
        camera_buffer: &render::UniformBuffer<gbase_utils::CameraUniform>,
        frustum_buffer: &render::UniformBuffer<CameraFrustum>,
        render_mode: RenderMode,
    ) {
        if !asset::handle_loaded(cache, self.draw_shader_handle.clone())
            || !asset::handle_loaded(cache, self.instance_shader_handle.clone())
            || !asset::handle_loaded(cache, self.render_forward_shader_handle.clone())
            || !asset::handle_loaded(cache, self.render_deferred_shader_handle.clone())
        {
            return;
        }

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
            encoder.push_debug_group(&format!("grass tile {i} buffer write"));
            // Alternate buffers to allow for GPU pipelining

            let mut i_cur = i % 2;
            if input::key_pressed(ctx, input::KeyCode::KeyP) {
                i_cur = 0;
            }

            self.instance_count[i_cur].write(ctx, &[0u32]);
            self.tile_buffer[i_cur].write(
                ctx,
                &Tile {
                    pos: tile,
                    size: TILE_SIZE,
                    blades_per_side: BLADES_PER_SIDE as f32,
                },
            );

            encoder.pop_debug_group();

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("grass compute {}", i)),
                timestamp_writes: None,
            });

            //
            // Instance
            //

            let instance_bindgroup = [
                render::BindGroupBuilder::new(self.instance_bindgroup_layout.clone())
                    .entries(vec![
                        // instances
                        render::BindGroupEntry::Buffer(self.instances[0].buffer()),
                        // instance count
                        render::BindGroupEntry::Buffer(self.instance_count[0].buffer()),
                        // tile
                        render::BindGroupEntry::Buffer(self.tile_buffer[0].buffer()),
                        // perlin texture
                        render::BindGroupEntry::Texture(self.perlin_noise_texture.view()),
                        // perlin texture sampler
                        render::BindGroupEntry::Sampler(self.perlin_noise_texture.sampler()),
                        // camera
                        render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                        // camera frustum
                        render::BindGroupEntry::Buffer(frustum_buffer.buffer()),
                        // app info
                        render::BindGroupEntry::Buffer(self.app_info.buffer()),
                        // debug input
                        render::BindGroupEntry::Buffer(self.debug_input.buffer()),
                    ])
                    .build(ctx),
                render::BindGroupBuilder::new(self.instance_bindgroup_layout.clone())
                    .label("instance")
                    .entries(vec![
                        // instances
                        render::BindGroupEntry::Buffer(self.instances[1].buffer()),
                        // instance count
                        render::BindGroupEntry::Buffer(self.instance_count[1].buffer()),
                        // tile
                        render::BindGroupEntry::Buffer(self.tile_buffer[1].buffer()),
                        // perlin texture
                        render::BindGroupEntry::Texture(self.perlin_noise_texture.view()),
                        // perlin texture sampler
                        render::BindGroupEntry::Sampler(self.perlin_noise_texture.sampler()),
                        // camera
                        render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                        // camera frustum
                        render::BindGroupEntry::Buffer(frustum_buffer.buffer()),
                        // app info
                        render::BindGroupEntry::Buffer(self.app_info.buffer()),
                        // debug input
                        render::BindGroupEntry::Buffer(self.debug_input.buffer()),
                    ])
                    .build(ctx),
            ];
            let instance_shader =
                asset::convert_asset(ctx, cache, self.instance_shader_handle.clone()).unwrap(); // TODO:

            let instance_pipeline = render::ComputePipelineBuilder::new(
                instance_shader,
                self.instance_pipeline_layout.clone(),
            )
            .label("instance".to_string())
            .build(ctx);

            compute_pass.set_pipeline(&instance_pipeline);
            compute_pass.set_bind_group(0, Some(instance_bindgroup[i_cur].as_ref()), &[]);
            compute_pass.dispatch_workgroups(BLADES_PER_SIDE / 16, BLADES_PER_SIDE / 16, 1);

            //
            // Draw indirect
            //

            let draw_bindgroup = [
                render::BindGroupBuilder::new(self.draw_bindgroup_layout.clone())
                    .label("draw")
                    .entries(vec![
                        // indirect buffer
                        render::BindGroupEntry::Buffer(self.indirect_buffer[0].buffer()),
                        // instance count
                        render::BindGroupEntry::Buffer(self.instance_count[0].buffer()),
                    ])
                    .build(ctx),
                render::BindGroupBuilder::new(self.draw_bindgroup_layout.clone())
                    .label("draw")
                    .entries(vec![
                        // indirect buffer
                        render::BindGroupEntry::Buffer(self.indirect_buffer[1].buffer()),
                        // instance count
                        render::BindGroupEntry::Buffer(self.instance_count[1].buffer()),
                    ])
                    .build(ctx),
            ];

            let draw_compute_shader =
                asset::convert_asset(ctx, cache, self.draw_shader_handle.clone()).unwrap(); // TODO:

            let draw_pipeline = render::ComputePipelineBuilder::new(
                draw_compute_shader,
                self.draw_pipeline_layout.clone(),
            )
            .label("draw".to_string())
            .build(ctx);

            compute_pass.set_pipeline(&draw_pipeline);
            compute_pass.set_bind_group(0, Some(draw_bindgroup[i_cur].as_ref()), &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);

            drop(compute_pass);

            //
            // Render
            //

            let render_bindgroup =
                render::BindGroupBuilder::new(self.render_bindgroup_layout.clone())
                    .label("render")
                    .entries(vec![
                        // Camera
                        render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                        // Debug
                        render::BindGroupEntry::Buffer(self.debug_input.buffer()),
                        // App info
                        render::BindGroupEntry::Buffer(self.app_info.buffer()),
                    ])
                    .build(ctx);

            match render_mode {
                RenderMode::Forward {
                    view,
                    view_format,
                    depth_buffer,
                } => {
                    let render_shader =
                        asset::convert_asset(ctx, cache, self.render_forward_shader_handle)
                            .unwrap();

                    let render_pipeline = render::RenderPipelineBuilder::new(
                        render_shader,
                        self.render_pipeline_layout.clone(),
                    )
                    .buffers(vec![GrassInstanceGPU::desc()])
                    .single_target(ColorTargetState::new().format(view_format))
                    .depth_stencil(depth_buffer.depth_stencil_state())
                    .topology(wgpu::PrimitiveTopology::TriangleStrip)
                    .build(ctx);

                    render::RenderPassBuilder::new()
                        .label(&format!("grass render {}", i))
                        .color_attachments(&[Some(RenderPassColorAttachment::new(view))])
                        .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
                        .build_run(&mut encoder, |mut render_pass| {
                            render_pass.set_pipeline(&render_pipeline);
                            render_pass.set_vertex_buffer(0, self.instances[i_cur].slice(..));
                            render_pass.set_bind_group(0, Some(render_bindgroup.as_ref()), &[]);
                            render_pass.draw_indirect(self.indirect_buffer[i_cur].buffer_ref(), 0);
                        });
                }
                RenderMode::Deferred { buffers } => {
                    let render_shader =
                        asset::convert_asset(ctx, cache, self.render_deferred_shader_handle)
                            .unwrap();
                    let render_pipeline = render::RenderPipelineBuilder::new(
                        render_shader,
                        self.render_pipeline_layout.clone(),
                    )
                    .buffers(vec![GrassInstanceGPU::desc()])
                    .multiple_targets(buffers.targets().to_vec())
                    .depth_stencil(buffers.depth.depth_stencil_state())
                    .topology(wgpu::PrimitiveTopology::TriangleStrip)
                    .build(ctx);

                    let attachments = &buffers.color_attachments();
                    render::RenderPassBuilder::new()
                        .color_attachments(attachments)
                        .depth_stencil_attachment(buffers.depth.depth_render_attachment_load())
                        .build_run(&mut encoder, |mut render_pass| {
                            render_pass.set_pipeline(&render_pipeline);
                            render_pass.set_vertex_buffer(0, self.instances[i_cur].slice(..));
                            render_pass.set_bind_group(0, Some(render_bindgroup.as_ref()), &[]);
                            render_pass.draw_indirect(self.indirect_buffer[i_cur].buffer_ref(), 0);
                        });
                }
            }

            let queue = render::queue(ctx);
            queue.submit(Some(encoder.finish()));
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
    pub fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: wgpu::vertex_attr_array![
                1=>Float32x3,   // pos
                2=>Uint32,      // hash
                3=>Float32x2,   // facing
                4=>Float32,     // wind
                5=>Float32,     // pad
                6=>Float32,     // height
                7=>Float32,     // tilt
                8=>Float32,     // bend
                9=>Float32,     // width
            ]
            .to_vec(),
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
