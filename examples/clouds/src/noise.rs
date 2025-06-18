use encase::ShaderType;
use gbase::{
    filesystem,
    render::{self},
    tracing, wgpu, Context,
};

#[derive(ShaderType)]
struct NoiseGeneratorUniforms {
    size: u32,
    cells_r: u32,
    cells_g: u32,
    cells_b: u32,
    perlin_scale: f32,
}

const NOISE_TEXTURE_DIM: u32 = 128;
const NOISE_UNIFORM: NoiseGeneratorUniforms = NoiseGeneratorUniforms {
    size: NOISE_TEXTURE_DIM,
    cells_r: 8,
    cells_g: 16,
    cells_b: 32,
    perlin_scale: 10.0,
};

pub fn generate_cloud_noise(ctx: &mut Context) -> Result<render::GpuImage, wgpu::Error> {
    // generate 3d texture
    let texture = render::TextureBuilder::new(render::TextureSource::Empty(
        NOISE_TEXTURE_DIM,
        NOISE_TEXTURE_DIM,
    ))
    .depth_or_array_layers(NOISE_TEXTURE_DIM)
    .with_format(wgpu::TextureFormat::Rgba8Unorm)
    .dimension(wgpu::TextureDimension::D3)
    .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
    .build(ctx);

    let sampler = render::SamplerBuilder::new()
        .with_address_mode(wgpu::AddressMode::Repeat)
        .build(ctx);
    let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
    let texture = render::GpuImage::new(texture, view, sampler);

    let noise_generator_info =
        render::UniformBufferBuilder::new(render::UniformBufferSource::Data(NOISE_UNIFORM))
            .build(ctx);

    let shader_str = filesystem::load_s!("shaders/cloud_noise.wgsl").unwrap();
    #[cfg(feature = "hot_reload")]
    let shader = render::ShaderBuilder::new(shader_str).build_err(ctx)?;
    #[cfg(not(feature = "hot_reload"))]
    let shader = render::ShaderBuilder::new(shader_str).build(ctx);

    let bindgroup_layout = render::BindGroupLayoutBuilder::new()
        .entries(vec![
            // app info
            render::BindGroupLayoutEntry::new().uniform().compute(),
            // output texture
            render::BindGroupLayoutEntry::new()
                .ty(wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D3,
                })
                .compute(),
        ])
        .build(ctx);

    let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
        .entries(vec![
            // app info
            render::BindGroupEntry::Buffer(noise_generator_info.buffer()),
            // output texture
            render::BindGroupEntry::Texture(texture.view()),
        ])
        .build(ctx);

    let compute_pipeline_layoyt = render::PipelineLayoutBuilder::new()
        .bind_groups(vec![bindgroup_layout])
        .build(ctx);
    let compute_pipeline =
        render::ComputePipelineBuilder::new(shader, compute_pipeline_layoyt).build(ctx);

    render::ComputePassBuilder::new().build_run_submit(ctx, |mut pass| {
        pass.set_pipeline(&compute_pipeline);
        pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
        pass.dispatch_workgroups(NOISE_TEXTURE_DIM, NOISE_TEXTURE_DIM, NOISE_TEXTURE_DIM);
    });

    Ok(texture)
}
