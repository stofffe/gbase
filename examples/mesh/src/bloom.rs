use gbase::{
    asset::{self, ShaderLoader},
    render::{self, FrameBuffer, FrameBufferBuilder},
    wgpu, Context,
};

pub struct Tonemap {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    shader_handle: asset::AssetHandle<render::ShaderBuilder>,
}

impl Tonemap {
    pub fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
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
        let shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            cache,
            "assets/shaders/tonemap.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);
        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
        }
    }

    pub fn tonemap(
        &self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        hdr_framebuffer: &render::FrameBuffer,
        ldr_framebuffer: &render::FrameBuffer,
    ) {
        if !asset::handle_loaded(cache, self.shader_handle.clone()) {
            return;
        }

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // in
                render::BindGroupEntry::Texture(hdr_framebuffer.view()),
                // out
                render::BindGroupEntry::Texture(ldr_framebuffer.view()),
            ])
            .build(ctx);

        let shader =
            asset::convert_asset::<wgpu::ShaderModule>(ctx, cache, self.shader_handle.clone())
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
    pub fn new(
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        buffer_format: wgpu::TextureFormat,
    ) -> Self {
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
        let extract_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            cache,
            "assets/shaders/bloom_extract.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);

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
        let downsample_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            cache,
            "assets/shaders/bloom_downsample.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);

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
        let upsample_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            cache,
            "assets/shaders/bloom_upsample.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);

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
        let combine_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
            cache,
            "assets/shaders/bloom_combine.wgsl",
            ShaderLoader {},
        )
        .watch(cache)
        .build(cache);

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
        cache: &mut gbase::asset::AssetCache,
        input_buffer: &render::FrameBuffer,
        output_buffer: &render::FrameBuffer,
    ) {
        if !asset::handle_loaded(cache, self.extract_shader_handle.clone())
            || !asset::handle_loaded(cache, self.combine_shader_handle.clone())
            || !asset::handle_loaded(cache, self.upsample_shader_handle.clone())
            || !asset::handle_loaded(cache, self.downsample_shader_handle.clone())
        {
            return;
        }

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
            asset::convert_asset(ctx, cache, self.extract_shader_handle.clone()).unwrap();
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
            .trace_gpu("extract")
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(
                &downsample_views[0],
            ))])
            .build_run(ctx, &mut encoder, |_ctx, mut pass| {
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
            cache,
            self.downsample_shader_handle.clone(),
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
                .build_run(ctx, &mut encoder, |_ctx, mut pass| {
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
            cache,
            self.upsample_shader_handle.clone(),
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
                .build_run(ctx, &mut encoder, |_ctx, mut pass| {
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
            cache,
            self.combine_shader_handle.clone(),
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
            .trace_gpu("combine")
            .build_run(ctx, &mut encoder, |_ctx, mut pass| {
                pass.set_pipeline(&combine_pipeline);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_bind_group(0, Some(combine_bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        encoder.submit(ctx);
    }
}
