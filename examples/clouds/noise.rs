use gbase::wgpu;
use gbase::{filesystem, render, Context};

const NOISE_TEXTURE_DIM: u32 = 128;

pub fn generate_noise(ctx: &mut Context) -> render::Texture {
    // generate 3d texture
    let texture = render::TextureBuilder::new(render::TextureSource::Empty(
        NOISE_TEXTURE_DIM,
        NOISE_TEXTURE_DIM,
    ))
    .format(wgpu::TextureFormat::Rgba8Unorm)
    .usage(wgpu::TextureUsages::STORAGE_BINDING)
    .build(ctx);

    let shader_str = filesystem::load_s!("shaders/cloud_noise.wgsl").unwrap();
    let shader = render::ShaderBuilder::new(shader_str).build(ctx);
    let (bindgroup_layout, bindgroup) = render::BindGroupCombinedBuilder::new()
        .entries(vec![
            // output texture
            render::BindGroupCombinedEntry::new(render::BindGroupEntry::Texture(texture.view()))
                .storage_texture_2d_write(wgpu::TextureFormat::Rgba8Unorm)
                .compute(),
        ])
        .build(ctx);
    let compute_pipeline_layoyt = render::PipelineLayoutBuilder::new()
        .bind_groups(vec![bindgroup_layout])
        .build(ctx);
    let compute_pipeline =
        render::ComputePipelineBuilder::new(shader, compute_pipeline_layoyt).build(ctx);

    render::ComputePassBuilder::new().build_run_new_encoder(ctx, |mut pass| {
        pass.set_pipeline(&compute_pipeline);
        pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
        pass.dispatch_workgroups(NOISE_TEXTURE_DIM, NOISE_TEXTURE_DIM, NOISE_TEXTURE_DIM);
    });

    texture
}
