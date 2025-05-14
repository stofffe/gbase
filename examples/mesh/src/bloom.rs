use gbase::{
    filesystem,
    render::{self, FrameBuffer, FrameBufferBuilder, UniformBufferBuilder},
    wgpu, Context,
};
use gbase_utils::{AssetCache, AssetHandle, GaussianFilterParams};

pub struct Tonemap {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    shader_handle: AssetHandle<render::ShaderBuilder>,
}

impl Tonemap {
    pub fn new(
        ctx: &mut Context,
        shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    ) -> Self {
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
        let shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder::new(filesystem::load_s!("shaders/tonemap.wgsl").unwrap()),
            "assets/shaders/tonemap.wgsl".into(),
        );
        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
        }
    }

    pub fn tonemap(
        &self,
        ctx: &mut Context,
        shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
        hdr_framebuffer: &render::FrameBuffer,
        ldr_framebuffer: &render::FrameBuffer,
    ) {
        debug_assert!(hdr_framebuffer.format() == wgpu::TextureFormat::Rgba16Float);
        debug_assert!(ldr_framebuffer.format() == wgpu::TextureFormat::Rgba8Unorm);

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // in
                render::BindGroupEntry::Texture(hdr_framebuffer.view()),
                // out
                render::BindGroupEntry::Texture(ldr_framebuffer.view()),
            ])
            .build(ctx);

        let shader = shader_cache.get_gpu(ctx, self.shader_handle.clone());
        let pipeline =
            render::ComputePipelineBuilder::new(shader, self.pipeline_layout.clone()).build(ctx);

        render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
            pass.dispatch_workgroups(ldr_framebuffer.width(), ldr_framebuffer.height(), 1);
        });
    }
}

// bloom steps
// in: HDR texture
// extract highlights
// blur hightlights
// combine highlights with in

pub struct Bloom {
    extract_pipeline_layout: render::ArcPipelineLayout,
    extract_bindgroup_layout: render::ArcBindGroupLayout,
    extract_shader_handle: AssetHandle<render::ShaderBuilder>,

    blur_pipeline_layout: render::ArcPipelineLayout,
    blur_bindgroup_layout: render::ArcBindGroupLayout,
    blur_shader_handle: AssetHandle<render::ShaderBuilder>,

    combine_pipeline_layout: render::ArcPipelineLayout,
    combine_bindgroup_layout: render::ArcBindGroupLayout,
    combine_shader_handle: AssetHandle<render::ShaderBuilder>,

    buffer1: FrameBuffer,
    buffer2: FrameBuffer,
}

impl Bloom {
    pub fn new(
        ctx: &mut Context,
        shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    ) -> Self {
        let extract_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // out
                render::BindGroupLayoutEntry::new()
                    .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
                    .compute(),
            ])
            .build(ctx);
        let extract_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![extract_bindgroup_layout.clone()])
            .build(ctx);
        let extract_shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder::new(filesystem::load_s!("shaders/bloom_extract.wgsl").unwrap()),
            "assets/shaders/bloom_extract.wgsl".into(),
        );

        let blur_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // out
                render::BindGroupLayoutEntry::new()
                    .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
                    .compute(),
            ])
            .build(ctx);
        let blur_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![blur_bindgroup_layout.clone()])
            .build(ctx);
        let blur_shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder::new(filesystem::load_s!("shaders/bloom_blur.wgsl").unwrap()),
            "assets/shaders/bloom_blur.wgsl".into(),
        );

        let combine_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // bloom
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // out
                render::BindGroupLayoutEntry::new()
                    .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
                    .compute(),
            ])
            .build(ctx);
        let combine_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![combine_bindgroup_layout.clone()])
            .build(ctx);
        let combine_shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder::new(filesystem::load_s!("shaders/bloom_combine.wgsl").unwrap()),
            "assets/shaders/bloom_combine.wgsl".into(),
        );

        let buffer1 = FrameBufferBuilder::new()
            .format(wgpu::TextureFormat::Rgba16Float)
            .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
            .screen_size(ctx)
            .build(ctx);
        let buffer2 = FrameBufferBuilder::new()
            .format(wgpu::TextureFormat::Rgba16Float)
            .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
            .screen_size(ctx)
            .build(ctx);

        Self {
            extract_pipeline_layout,
            extract_bindgroup_layout,
            extract_shader_handle,

            blur_pipeline_layout,
            blur_bindgroup_layout,
            blur_shader_handle,

            combine_pipeline_layout,
            combine_bindgroup_layout,
            combine_shader_handle,

            buffer1,
            buffer2,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
        input_buffer: &render::FrameBuffer,
        output_buffer: &render::FrameBuffer,
    ) {
        debug_assert!(input_buffer.format() == wgpu::TextureFormat::Rgba16Float);
        debug_assert!(output_buffer.format() == wgpu::TextureFormat::Rgba16Float);

        self.buffer1.resize(ctx, input_buffer.size());
        self.buffer2.resize(ctx, input_buffer.size());

        //
        // extract
        //
        let extract_bindgroup =
            render::BindGroupBuilder::new(self.extract_bindgroup_layout.clone())
                .entries(vec![
                    // in
                    render::BindGroupEntry::Texture(input_buffer.view()),
                    // out
                    render::BindGroupEntry::Texture(self.buffer1.view()),
                ])
                .build(ctx);

        let extract_shader = shader_cache.get_gpu(ctx, self.extract_shader_handle.clone());
        let extract_pipeline = render::ComputePipelineBuilder::new(
            extract_shader,
            self.extract_pipeline_layout.clone(),
        )
        .build(ctx);

        render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
            pass.set_pipeline(&extract_pipeline);
            pass.set_bind_group(0, Some(extract_bindgroup.as_ref()), &[]);
            pass.dispatch_workgroups(output_buffer.width(), output_buffer.height(), 1);
        });

        // return

        //
        // blur
        //
        let blur_horizontal_bindgroup =
            render::BindGroupBuilder::new(self.blur_bindgroup_layout.clone())
                .entries(vec![
                    // in
                    render::BindGroupEntry::Texture(self.buffer1.view()),
                    // out
                    render::BindGroupEntry::Texture(self.buffer2.view()),
                ])
                .build(ctx);
        let blur_horizontal_shader = shader_cache.get_gpu(ctx, self.blur_shader_handle.clone());
        let blur_horizontal_pipeline = render::ComputePipelineBuilder::new(
            blur_horizontal_shader,
            self.blur_pipeline_layout.clone(),
        )
        .entry_point("horizontal")
        .build(ctx);
        let blur_vertical_bindgroup =
            render::BindGroupBuilder::new(self.blur_bindgroup_layout.clone())
                .entries(vec![
                    // in
                    render::BindGroupEntry::Texture(self.buffer2.view()),
                    // out
                    render::BindGroupEntry::Texture(self.buffer1.view()),
                ])
                .build(ctx);
        let blur_vertical_shader = shader_cache.get_gpu(ctx, self.blur_shader_handle.clone());
        let blur_vertical_pipeline = render::ComputePipelineBuilder::new(
            blur_vertical_shader,
            self.blur_pipeline_layout.clone(),
        )
        .entry_point("vertical")
        .build(ctx);

        let width = output_buffer.width().div_ceil(16);
        let height = output_buffer.height().div_ceil(16);
        for _ in 0..5 {
            render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&blur_horizontal_pipeline);
                pass.set_bind_group(0, Some(blur_horizontal_bindgroup.as_ref()), &[]);
                pass.dispatch_workgroups(width, height, 1);
            });
            render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&blur_vertical_pipeline);
                pass.set_bind_group(0, Some(blur_vertical_bindgroup.as_ref()), &[]);
                pass.dispatch_workgroups(width, height, 1);
            });
        }

        //
        // combine
        //

        let combine_bindgroup =
            render::BindGroupBuilder::new(self.combine_bindgroup_layout.clone())
                .entries(vec![
                    // in
                    render::BindGroupEntry::Texture(input_buffer.view()),
                    // bloom
                    render::BindGroupEntry::Texture(self.buffer1.view()),
                    // out
                    render::BindGroupEntry::Texture(output_buffer.view()),
                ])
                .build(ctx);

        let combine_shader = shader_cache.get_gpu(ctx, self.combine_shader_handle.clone());
        let combine_pipeline = render::ComputePipelineBuilder::new(
            combine_shader,
            self.combine_pipeline_layout.clone(),
        )
        .build(ctx);

        render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
            pass.set_pipeline(&combine_pipeline);
            pass.set_bind_group(0, Some(combine_bindgroup.as_ref()), &[]);
            pass.dispatch_workgroups(output_buffer.width(), output_buffer.height(), 1);
        });
    }

    // pub fn extract_bright_pixels(
    //     &self,
    //     ctx: &mut Context,
    //     shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    //     input_buffer: &render::FrameBuffer,
    //     output_buffer: &render::FrameBuffer,
    // ) {
    //     debug_assert!(input_buffer.format() == wgpu::TextureFormat::Rgba16Float);
    //     debug_assert!(output_buffer.format() == wgpu::TextureFormat::Rgba16Float);
    //
    //     let bindgroup = render::BindGroupBuilder::new(self.extract_bindgroup_layout.clone())
    //         .entries(vec![
    //             // in
    //             render::BindGroupEntry::Texture(input_buffer.view()),
    //             // out
    //             render::BindGroupEntry::Texture(output_buffer.view()),
    //             // params
    //         ])
    //         .build(ctx);
    //
    //     let shader = shader_cache.get_gpu(ctx, self.extract_shader_handle.clone());
    //     let pipeline =
    //         render::ComputePipelineBuilder::new(shader, self.extract_pipeline_layout.clone())
    //             .build(ctx);
    //
    //     render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
    //         pass.set_pipeline(&pipeline);
    //         pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
    //         pass.dispatch_workgroups(output_buffer.width(), output_buffer.height(), 1);
    //     });
    // }
    //
    // pub fn blur_bright_pixels() {}
}

// pub struct GaussianBlur {
//     pipeline_layout: render::ArcPipelineLayout,
//     bindgroup_layout: render::ArcBindGroupLayout,
//     shader_handle: AssetHandle<render::ShaderBuilder>,
//
//     params_buffer: render::UniformBuffer<GaussianFilterParams>,
// }
//
// impl GaussianBlur {
//     pub fn new(
//         ctx: &mut Context,
//         shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
//     ) -> Self {
//         let bindgroup_layout = render::BindGroupLayoutBuilder::new()
//             .entries(vec![
//                 // in
//                 render::BindGroupLayoutEntry::new()
//                     .texture_float_filterable()
//                     .compute(),
//                 // out
//                 render::BindGroupLayoutEntry::new()
//                     .storage_texture_2d_write(wgpu::TextureFormat::Rgba16Float)
//                     .compute(),
//                 // params
//                 render::BindGroupLayoutEntry::new().uniform().compute(),
//             ])
//             .build(ctx);
//         let pipeline_layout = render::PipelineLayoutBuilder::new()
//             .bind_groups(vec![bindgroup_layout.clone()])
//             .build(ctx);
//         let shader_handle = shader_cache.allocate_reload(
//             render::ShaderBuilder::new(filesystem::load_s!("shaders/gaussian_blur.wgsl").unwrap()),
//             "assets/shaders/gaussian_blur.wgsl".into(),
//         );
//
//         let params_buffer =
//             UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
//
//         Self {
//             pipeline_layout,
//             bindgroup_layout,
//             shader_handle,
//
//             params_buffer,
//         }
//     }
//
//     pub fn blur_dual_pass(
//         &self,
//         ctx: &mut Context,
//         shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
//
//         input_buffer: &render::FrameBuffer,
//         output_buffer: &render::FrameBuffer,
//
//         params: GaussianFilterParams,
//     ) {
//         debug_assert!(input_buffer.format() == wgpu::TextureFormat::Rgba16Float);
//         debug_assert!(output_buffer.format() == wgpu::TextureFormat::Rgba16Float);
//
//         self.params_buffer.write(ctx, &params);
//         let shader = shader_cache.get_gpu(ctx, self.shader_handle.clone());
//
//         // horizontal
//         let bindgroup_h = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
//             .entries(vec![
//                 // in
//                 render::BindGroupEntry::Texture(input_buffer.view()),
//                 // out
//                 render::BindGroupEntry::Texture(output_buffer.view()),
//                 // params
//                 render::BindGroupEntry::Buffer(self.params_buffer.buffer()),
//             ])
//             .build(ctx);
//         let pipeline_h =
//             render::ComputePipelineBuilder::new(shader.clone(), self.pipeline_layout.clone())
//                 .entry_point("horizontal")
//                 .build(ctx);
//
//         // vertical
//         let bindgroup_v = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
//             .entries(vec![
//                 // in
//                 render::BindGroupEntry::Texture(output_buffer.view()),
//                 // out
//                 render::BindGroupEntry::Texture(input_buffer.view()),
//                 // params
//                 render::BindGroupEntry::Buffer(self.params_buffer.buffer()),
//             ])
//             .build(ctx);
//         let pipeline_v = render::ComputePipelineBuilder::new(shader, self.pipeline_layout.clone())
//             .entry_point("vertical")
//             .build(ctx);
//
//         let width = output_buffer.width().div_ceil(16);
//         let height = output_buffer.height().div_ceil(16);
//
//         let mut encoder = render::EncoderBuilder::new().build(ctx);
//         render::ComputePassBuilder::new().build_run(&mut encoder, |mut pass| {
//             pass.set_pipeline(&pipeline_h);
//             pass.set_bind_group(0, Some(bindgroup_h.as_ref()), &[]);
//             pass.dispatch_workgroups(width, height, 1);
//         });
//         render::ComputePassBuilder::new().build_run(&mut encoder, |mut pass| {
//             pass.set_pipeline(&pipeline_v);
//             pass.set_bind_group(0, Some(bindgroup_v.as_ref()), &[]);
//             pass.dispatch_workgroups(width, height, 1);
//         });
//         let queue = render::queue(ctx);
//         queue.submit(Some(encoder.finish()));
//     }
// }
