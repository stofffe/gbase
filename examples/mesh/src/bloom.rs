use gbase::{
    asset, filesystem,
    render::{self, FrameBuffer, FrameBufferBuilder},
    wgpu, Context,
};

pub struct Tonemap {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    shader_handle: asset::AssetHandle<render::ShaderBuilder>,
}

impl Tonemap {
    pub fn new(ctx: &mut Context) -> Self {
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // out
                render::BindGroupLayoutEntry::new()
                    .storage_texture_2d_write(wgpu::TextureFormat::Rgba8Unorm)
                    .compute(),
            ])
            .build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let shader_handle = asset::AssetBuilder::load("assets/shaders/tonemap.wgsl")
            .watch(ctx)
            .build(ctx);
        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
        }
    }

    pub fn tonemap(
        &self,
        ctx: &mut Context,
        hdr_framebuffer: &render::FrameBuffer,
        ldr_framebuffer: &render::FrameBuffer,
    ) {
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // in
                render::BindGroupEntry::Texture(hdr_framebuffer.view()),
                // out
                render::BindGroupEntry::Texture(ldr_framebuffer.view()),
            ])
            .build(ctx);

        let shader =
            asset::convert_asset::<wgpu::ShaderModule>(ctx, self.shader_handle.clone(), &())
                .unwrap();
        let pipeline =
            render::ComputePipelineBuilder::new(shader, self.pipeline_layout.clone()).build(ctx);

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::ComputePassBuilder::new()
            // .timestamp_writes(render::gpu_profiler(ctx).profile_compute_pass("tonemap"))
            .trace_gpu(ctx, "tonemap")
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                pass.dispatch_workgroups(ldr_framebuffer.width(), ldr_framebuffer.height(), 1);
            });

        render::queue(ctx).submit([encoder.finish()]);
    }
}

pub struct Bloom {
    extract_pipeline_layout: render::ArcPipelineLayout,
    extract_bindgroup_layout: render::ArcBindGroupLayout,
    extract_shader_handle: asset::AssetHandle<render::ShaderBuilder>,

    downsample_pipeline_layout: render::ArcPipelineLayout,
    downsample_bindgroup_layout: render::ArcBindGroupLayout,
    downsample_shader_handle: asset::AssetHandle<render::ShaderBuilder>,

    upsample_pipeline_layout: render::ArcPipelineLayout,
    upsample_bindgroup_layout: render::ArcBindGroupLayout,
    upsample_shader_handle: asset::AssetHandle<render::ShaderBuilder>,

    combine_pipeline_layout: render::ArcPipelineLayout,
    combine_bindgroup_layout: render::ArcBindGroupLayout,
    combine_shader_handle: asset::AssetHandle<render::ShaderBuilder>,

    downsampling_buffer: FrameBuffer,
    upsampling_buffer: FrameBuffer,

    vertices: render::VertexBuffer<render::VertexUV>,
    indices: render::IndexBuffer,

    black_pixel: render::GpuImage,
}

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right

    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

];

#[rustfmt::skip]
const CENTERED_QUAD_INDICES: &[u32] = &[
    0, 1, 2,
    3, 4, 5
];

const MIP_LEVELS: u32 = 5;

impl Bloom {
    pub fn new(ctx: &mut Context, buffer_format: wgpu::TextureFormat) -> Self {
        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            CENTERED_QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);
        let indices = render::IndexBufferBuilder::new(render::IndexBufferSource::Data(
            CENTERED_QUAD_INDICES.to_vec(),
        ))
        .build(ctx);

        //
        // Extract
        //

        let extract_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let extract_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![extract_bindgroup_layout.clone()])
            .build(ctx);
        let extract_shader_handle = asset::AssetBuilder::load("assets/shaders/bloom_extract.wgsl")
            .watch(ctx)
            .build(ctx);

        //
        // Downsample
        //

        let downsample_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let downsample_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![downsample_bindgroup_layout.clone()])
            .build(ctx);
        let downsample_shader_handle =
            asset::AssetBuilder::load("assets/shaders/bloom_downsample.wgsl")
                .watch(ctx)
                .build(ctx);

        //
        // Upsample
        //

        let upsample_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // previous
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let upsample_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![upsample_bindgroup_layout.clone()])
            .build(ctx);
        let upsample_shader_handle =
            asset::AssetBuilder::load("assets/shaders/bloom_upsample.wgsl")
                .watch(ctx)
                .build(ctx);

        //
        // Combine
        //

        let combine_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // bloom 0
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // bloom 1
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // bloom 2
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // bloom 3
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // bloom 4
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let combine_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![combine_bindgroup_layout.clone()])
            .build(ctx);
        let combine_shader_handle = asset::AssetBuilder::load("assets/shaders/bloom_combine.wgsl")
            .watch(ctx)
            .build(ctx);

        let downsampling_buffer = FrameBufferBuilder::new()
            .label("downsampling")
            .format(buffer_format)
            .usage(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT)
            .mip_level_count(MIP_LEVELS)
            .screen_size(ctx)
            .build(ctx);
        let upsampling_buffer = FrameBufferBuilder::new()
            .label("upsampling")
            .format(buffer_format)
            .usage(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT)
            .mip_level_count(MIP_LEVELS)
            .screen_size(ctx)
            .build(ctx);

        let image = render::Image::new_pixel_texture([0, 0, 0, 0]);
        let texture = image.texture.clone().build(ctx);
        let sampler = image.sampler.clone().build(ctx);
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
        let black_pixel = render::GpuImage::new(texture, view, sampler);

        Self {
            extract_pipeline_layout,
            extract_bindgroup_layout,
            extract_shader_handle,

            downsample_pipeline_layout,
            downsample_bindgroup_layout,
            downsample_shader_handle,

            upsample_pipeline_layout,
            upsample_bindgroup_layout,
            upsample_shader_handle,

            combine_pipeline_layout,
            combine_bindgroup_layout,
            combine_shader_handle,

            downsampling_buffer,
            upsampling_buffer,

            vertices,
            indices,
            black_pixel,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        input_buffer: &render::FrameBuffer,
        output_buffer: &render::FrameBuffer,
    ) {
        self.downsampling_buffer.resize(ctx, input_buffer.size());
        self.upsampling_buffer.resize(ctx, input_buffer.size());

        let mut encoder = render::EncoderBuilder::new().build_new(ctx);

        let mut downsample_views = Vec::new();
        let mut upsample_views = Vec::new();
        for i in 0..MIP_LEVELS {
            downsample_views.push(
                render::TextureViewBuilder::new(self.downsampling_buffer.texture())
                    .base_mip_level(i)
                    .mip_level_count(1)
                    .build(ctx),
            );
            upsample_views.push(
                render::TextureViewBuilder::new(self.upsampling_buffer.texture())
                    .base_mip_level(i)
                    .mip_level_count(1)
                    .build(ctx),
            );
        }

        //
        // extract
        //

        let extract_sampler = render::SamplerBuilder::new().build(ctx);
        let extract_bindgroup =
            render::BindGroupBuilder::new(self.extract_bindgroup_layout.clone())
                .entries(vec![
                    // texture
                    render::BindGroupEntry::Texture(input_buffer.view()),
                    // sampler
                    render::BindGroupEntry::Sampler(extract_sampler.clone()),
                ])
                .build(ctx);

        let extract_shader =
            asset::convert_asset(ctx, self.extract_shader_handle.clone(), &()).unwrap();
        let extract_pipeline = render::RenderPipelineBuilder::new(
            extract_shader,
            self.extract_pipeline_layout.clone(),
        )
        .label("extract")
        .buffers(vec![self.vertices.desc()])
        .single_target(render::ColorTargetState::from_framebuffer(
            &self.downsampling_buffer,
        ))
        .build(ctx);

        render::RenderPassBuilder::new()
            .label("extract")
            .trace_gpu(ctx, "extract")
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(
                &downsample_views[0],
            ))])
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&extract_pipeline);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_bind_group(0, Some(extract_bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        //
        // downsample
        //

        let downsample_sampler = render::SamplerBuilder::new()
            .mip_map_filer(wgpu::FilterMode::Linear)
            .with_address_mode(wgpu::AddressMode::ClampToEdge)
            .build(ctx);
        let downsample_shader = asset::convert_asset::<wgpu::ShaderModule>(
            ctx,
            self.downsample_shader_handle.clone(),
            &(),
        )
        .unwrap();
        let downsample_pipeline = render::RenderPipelineBuilder::new(
            downsample_shader.clone(),
            self.downsample_pipeline_layout.clone(),
        )
        .label("downsample")
        .buffers(vec![self.vertices.desc()])
        .single_target(render::ColorTargetState::from_framebuffer(
            &self.downsampling_buffer,
        ))
        .build(ctx);
        for i in 1..MIP_LEVELS as usize {
            let downsample_bindgroup =
                render::BindGroupBuilder::new(self.downsample_bindgroup_layout.clone())
                    .label("downsample")
                    .entries(vec![
                        // texture
                        render::BindGroupEntry::Texture(downsample_views[i - 1].clone()),
                        // sampler
                        render::BindGroupEntry::Sampler(downsample_sampler.clone()),
                    ])
                    .build(ctx);
            render::RenderPassBuilder::new()
                .label(&format!("downsample {} -> {}", i - 1, i))
                .color_attachments(&[Some(render::RenderPassColorAttachment::new(
                    &downsample_views[i],
                ))])
                .build_run(&mut encoder, |mut pass| {
                    pass.set_pipeline(&downsample_pipeline);
                    pass.set_vertex_buffer(0, self.vertices.slice(..));
                    pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.set_bind_group(0, Some(downsample_bindgroup.as_ref()), &[]);
                    pass.draw_indexed(0..self.indices.len(), 0, 0..1);
                });
        }

        //
        // upsample
        //

        let upsample_sampler = render::SamplerBuilder::new()
            .mip_map_filer(wgpu::FilterMode::Linear)
            .with_address_mode(wgpu::AddressMode::ClampToEdge)
            .build(ctx);
        let upsample_shader = asset::convert_asset::<wgpu::ShaderModule>(
            ctx,
            self.upsample_shader_handle.clone(),
            &(),
        )
        .unwrap();
        let upsample_pipeline = render::RenderPipelineBuilder::new(
            upsample_shader.clone(),
            self.upsample_pipeline_layout.clone(),
        )
        .label("upsample")
        .buffers(vec![self.vertices.desc()])
        .single_target(render::ColorTargetState::from_framebuffer(
            &self.upsampling_buffer,
        ))
        .build(ctx);

        for i in (0..MIP_LEVELS as usize).rev() {
            let upsample_bindgroup =
                render::BindGroupBuilder::new(self.upsample_bindgroup_layout.clone())
                    .label("upsample")
                    .entries(vec![
                        // texture
                        render::BindGroupEntry::Texture(downsample_views[i].clone()),
                        //previous
                        render::BindGroupEntry::Texture(if i + 1 == MIP_LEVELS as usize {
                            self.black_pixel.view() // TODO: temp
                        } else {
                            upsample_views[i + 1].clone()
                        }),
                        // sampler
                        render::BindGroupEntry::Sampler(upsample_sampler.clone()),
                    ])
                    .build(ctx);
            render::RenderPassBuilder::new()
                .label(&format!("upsample {}", i))
                .color_attachments(&[Some(
                    render::RenderPassColorAttachment::new(&upsample_views[i])
                        .clear(wgpu::Color::BLACK),
                )])
                .build_run(&mut encoder, |mut pass| {
                    pass.set_pipeline(&upsample_pipeline);
                    pass.set_vertex_buffer(0, self.vertices.slice(..));
                    pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.set_bind_group(0, Some(upsample_bindgroup.as_ref()), &[]);
                    pass.draw_indexed(0..self.indices.len(), 0, 0..1);
                });
        }

        //
        // combine
        //

        let combine_sampler = render::SamplerBuilder::new()
            .with_address_mode(wgpu::AddressMode::ClampToEdge)
            .build(ctx);
        let combine_bindgroup =
            render::BindGroupBuilder::new(self.combine_bindgroup_layout.clone())
                .entries(vec![
                    // in
                    render::BindGroupEntry::Texture(input_buffer.view()),
                    // bloom 0
                    render::BindGroupEntry::Texture(upsample_views[0].clone()),
                    // bloom 1
                    render::BindGroupEntry::Texture(upsample_views[1].clone()),
                    // bloom 2
                    render::BindGroupEntry::Texture(upsample_views[2].clone()),
                    // bloom 3
                    render::BindGroupEntry::Texture(upsample_views[3].clone()),
                    // bloom 4
                    render::BindGroupEntry::Texture(upsample_views[4].clone()),
                    // sampler
                    render::BindGroupEntry::Sampler(combine_sampler.clone()),
                ])
                .build(ctx);

        let combine_shader = asset::convert_asset::<wgpu::ShaderModule>(
            ctx,
            self.combine_shader_handle.clone(),
            &(),
        )
        .unwrap();
        let combine_pipeline = render::RenderPipelineBuilder::new(
            combine_shader,
            self.combine_pipeline_layout.clone(),
        )
        .buffers(vec![self.vertices.desc()])
        .single_target(render::ColorTargetState::from_framebuffer(output_buffer))
        .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(output_buffer.view_ref())
                    .clear(wgpu::Color::BLACK),
            )])
            .label("combine")
            .trace_gpu(ctx, "combine")
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&combine_pipeline);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_bind_group(0, Some(combine_bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        encoder.submit(ctx);
    }
}

// impl Bloom {
//     pub fn new(
//         ctx: &mut Context,
//         shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
//     ) -> Self {
//         let extract_bindgroup_layout = render::BindGroupLayoutBuilder::new()
//             .entries(vec![
//                 // in
//                 render::BindGroupLayoutEntry::new()
//                     .texture_float_filterable()
//                     .compute(),
//                 // out
//                 render::BindGroupLayoutEntry::new()
//                     .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
//                     .compute(),
//             ])
//             .build(ctx);
//         let extract_pipeline_layout = render::PipelineLayoutBuilder::new()
//             .bind_groups(vec![extract_bindgroup_layout.clone()])
//             .build(ctx);
//         let extract_shader_handle = shader_cache.allocate_reload(
//             render::ShaderBuilder::new(filesystem::load_s!("shaders/bloom_extract.wgsl").unwrap()),
//             "assets/shaders/bloom_extract.wgsl".into(),
//         );
//
//         let blur_bindgroup_layout = render::BindGroupLayoutBuilder::new()
//             .entries(vec![
//                 // in
//                 render::BindGroupLayoutEntry::new()
//                     .texture_float_filterable()
//                     .compute(),
//                 // out
//                 render::BindGroupLayoutEntry::new()
//                     .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
//                     .compute(),
//             ])
//             .build(ctx);
//         let blur_pipeline_layout = render::PipelineLayoutBuilder::new()
//             .bind_groups(vec![blur_bindgroup_layout.clone()])
//             .build(ctx);
//         let blur_shader_handle = shader_cache.allocate_reload(
//             render::ShaderBuilder::new(filesystem::load_s!("shaders/bloom_blur.wgsl").unwrap()),
//             "assets/shaders/bloom_blur.wgsl".into(),
//         );
//
//         let combine_bindgroup_layout = render::BindGroupLayoutBuilder::new()
//             .entries(vec![
//                 // in
//                 render::BindGroupLayoutEntry::new()
//                     .texture_float_filterable()
//                     .compute(),
//                 // bloom
//                 render::BindGroupLayoutEntry::new()
//                     .texture_float_filterable()
//                     .compute(),
//                 // out
//                 render::BindGroupLayoutEntry::new()
//                     .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
//                     .compute(),
//             ])
//             .build(ctx);
//         let combine_pipeline_layout = render::PipelineLayoutBuilder::new()
//             .bind_groups(vec![combine_bindgroup_layout.clone()])
//             .build(ctx);
//         let combine_shader_handle = shader_cache.allocate_reload(
//             render::ShaderBuilder::new(filesystem::load_s!("shaders/bloom_combine.wgsl").unwrap()),
//             "assets/shaders/bloom_combine.wgsl".into(),
//         );
//
//         let buffer1 = FrameBufferBuilder::new()
//             .format(wgpu::TextureFormat::Rgba16Float)
//             .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
//             .screen_size(ctx)
//             .build(ctx);
//         let buffer2 = FrameBufferBuilder::new()
//             .format(wgpu::TextureFormat::Rgba16Float)
//             .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
//             .screen_size(ctx)
//             .build(ctx);
//
//         let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
//             CENTERED_QUAD_VERTICES.to_vec(),
//         ))
//         .build(ctx);
//         let indices = render::IndexBufferBuilder::new(render::IndexBufferSource::Data(
//             CENTERED_QUAD_INDICES.to_vec(),
//         ))
//         .build(ctx);
//
//         Self {
//             extract_pipeline_layout,
//             extract_bindgroup_layout,
//             extract_shader_handle,
//
//             blur_pipeline_layout,
//             blur_bindgroup_layout,
//             blur_shader_handle,
//
//             combine_pipeline_layout,
//             combine_bindgroup_layout,
//             combine_shader_handle,
//
//             buffer1,
//             buffer2,
//
//             vertices,
//             indices,
//         }
//     }
//
//     pub fn render(
//         &mut self,
//         ctx: &mut Context,
//         shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
//         input_buffer: &render::FrameBuffer,
//         output_buffer: &render::FrameBuffer,
//     ) {
//         debug_assert!(input_buffer.format() == wgpu::TextureFormat::Rgba16Float);
//         debug_assert!(output_buffer.format() == wgpu::TextureFormat::Rgba16Float);
//
//         self.buffer1.resize(ctx, input_buffer.size());
//         self.buffer2.resize(ctx, input_buffer.size());
//
//         //
//         // extract
//         //
//
//         let mut encoder = render::EncoderBuilder::new().build_new(ctx);
//
//         let extract_bindgroup =
//             render::BindGroupBuilder::new(self.extract_bindgroup_layout.clone())
//                 .entries(vec![
//                     // in
//                     render::BindGroupEntry::Texture(input_buffer.view()),
//                     // out
//                     render::BindGroupEntry::Texture(self.buffer1.view()),
//                 ])
//                 .build(ctx);
//
//         let extract_shader = shader_cache.get_gpu(ctx, self.extract_shader_handle.clone());
//         let extract_pipeline = render::ComputePipelineBuilder::new(
//             extract_shader,
//             self.extract_pipeline_layout.clone(),
//         )
//         .build(ctx);
//
//         render::ComputePassBuilder::new()
//             .label("extract")
//             .trace_gpu(ctx, "extract")
//             // .timestamp_writes(render::gpu_profiler(ctx).profile_compute_pass("extract"))
//             .build_run(&mut encoder, |mut pass| {
//                 pass.set_pipeline(&extract_pipeline);
//                 pass.set_bind_group(0, Some(extract_bindgroup.as_ref()), &[]);
//                 pass.dispatch_workgroups(
//                     self.buffer1.width().div_ceil(16),
//                     self.buffer1.height().div_ceil(16),
//                     1,
//                 );
//             });
//
//         //
//         // blur
//         //
//         let blur_horizontal_bindgroup =
//             render::BindGroupBuilder::new(self.blur_bindgroup_layout.clone())
//                 .entries(vec![
//                     // in
//                     render::BindGroupEntry::Texture(self.buffer1.view()),
//                     // out
//                     render::BindGroupEntry::Texture(self.buffer2.view()),
//                 ])
//                 .build(ctx);
//         let blur_horizontal_shader = shader_cache.get_gpu(ctx, self.blur_shader_handle.clone());
//         let blur_horizontal_pipeline = render::ComputePipelineBuilder::new(
//             blur_horizontal_shader,
//             self.blur_pipeline_layout.clone(),
//         )
//         .entry_point("horizontal")
//         .build(ctx);
//         let blur_vertical_bindgroup =
//             render::BindGroupBuilder::new(self.blur_bindgroup_layout.clone())
//                 .entries(vec![
//                     // in
//                     render::BindGroupEntry::Texture(self.buffer2.view()),
//                     // out
//                     render::BindGroupEntry::Texture(self.buffer1.view()),
//                 ])
//                 .build(ctx);
//         let blur_vertical_shader = shader_cache.get_gpu(ctx, self.blur_shader_handle.clone());
//         let blur_vertical_pipeline = render::ComputePipelineBuilder::new(
//             blur_vertical_shader,
//             self.blur_pipeline_layout.clone(),
//         )
//         .entry_point("vertical")
//         .build(ctx);
//
//         render::ComputePassBuilder::new()
//             .label("blur")
//             // .timestamp_writes(render::gpu_profiler(ctx).profile_compute_pass("blur"))
//             .trace_gpu(ctx, "blur")
//             .build_run(&mut encoder, |mut pass| {
//                 for _ in 0..5 {
//                     pass.set_pipeline(&blur_horizontal_pipeline);
//                     pass.set_bind_group(0, Some(blur_horizontal_bindgroup.as_ref()), &[]);
//                     pass.dispatch_workgroups(
//                         self.buffer2.width().div_ceil(16),
//                         self.buffer2.height().div_ceil(16),
//                         1,
//                     );
//
//                     pass.set_pipeline(&blur_vertical_pipeline);
//                     pass.set_bind_group(0, Some(blur_vertical_bindgroup.as_ref()), &[]);
//                     pass.dispatch_workgroups(
//                         self.buffer1.width().div_ceil(16),
//                         self.buffer1.height().div_ceil(16),
//                         1,
//                     );
//                 }
//             });
//
//         //
//         // combine
//         //
//
//         let combine_bindgroup =
//             render::BindGroupBuilder::new(self.combine_bindgroup_layout.clone())
//                 .entries(vec![
//                     // in
//                     render::BindGroupEntry::Texture(input_buffer.view()),
//                     // bloom
//                     render::BindGroupEntry::Texture(self.buffer1.view()),
//                     // out
//                     render::BindGroupEntry::Texture(output_buffer.view()),
//                 ])
//                 .build(ctx);
//
//         let combine_shader = shader_cache.get_gpu(ctx, self.combine_shader_handle.clone());
//         let combine_pipeline = render::ComputePipelineBuilder::new(
//             combine_shader,
//             self.combine_pipeline_layout.clone(),
//         )
//         .build(ctx);
//
//         render::ComputePassBuilder::new()
//             .label("combine")
//             // .timestamp_writes(render::gpu_profiler(ctx).profile_compute_pass("combine"))
//             .trace_gpu(ctx, "combine")
//             .build_run(&mut encoder, |mut pass| {
//                 pass.set_pipeline(&combine_pipeline);
//                 pass.set_bind_group(0, Some(combine_bindgroup.as_ref()), &[]);
//                 pass.dispatch_workgroups(
//                     output_buffer.width().div_ceil(16),
//                     output_buffer.height().div_ceil(16),
//                     1,
//                 );
//             });
//
//         encoder.submit(ctx);
//     }
// }
