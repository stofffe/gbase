use gbase::{filesystem, render, wgpu, Context};
use gbase_utils::{AssetCache, AssetHandle};

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
