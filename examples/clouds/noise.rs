use encase::ShaderType;
use gbase::{filesystem, render, wgpu, Context};

const NOISE_TEXTURE_DIM: u32 = 128;
const NOISE_UNIFORM: NoiseGeneratorUniforms = NoiseGeneratorUniforms {
    size: NOISE_TEXTURE_DIM,
    cells_r: 16,
    cells_g: 8,
    cells_b: 4,
    cells_a: 4,
};

pub fn generate_noise(ctx: &mut Context) -> Option<render::Texture> {
    // generate 3d texture
    let texture = render::TextureBuilder::new(render::TextureSource::Empty(
        NOISE_TEXTURE_DIM,
        NOISE_TEXTURE_DIM,
    ))
    .depth_or_array_layers(NOISE_TEXTURE_DIM)
    .format(wgpu::TextureFormat::Rgba8Unorm)
    .dimension(wgpu::TextureDimension::D3)
    .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
    .build(ctx);

    let noise_generator_info =
        render::UniformBufferBuilder::new(render::UniformBufferSource::Data(NOISE_UNIFORM))
            .build(ctx);

    let shader_str = filesystem::load_s!("shaders/cloud_noise.wgsl").unwrap();
    let shader = render::ShaderBuilder::new(shader_str)
        .build_unchached_err(ctx)
        .ok()?;

    let (bindgroup_layout, bindgroup) = render::BindGroupCombinedBuilder::new()
        .entries(vec![
            // app info
            render::BindGroupCombinedEntry::new(render::BindGroupEntry::Buffer(
                noise_generator_info.buffer(),
            ))
            .uniform()
            .compute(),
            // output texture
            render::BindGroupCombinedEntry::new(render::BindGroupEntry::Texture(texture.view()))
                .ty(wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D3,
                })
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

    Some(texture)
}

#[derive(ShaderType)]
struct NoiseGeneratorUniforms {
    size: u32,
    cells_r: u32,
    cells_g: u32,
    cells_b: u32,
    cells_a: u32,
}
