use gbase::{
    filesystem,
    input::{self, KeyCode},
    render::{self, ArcTextureView},
    wgpu, winit, Callbacks, Context,
};
use gbase_utils::{box_filter, gaussian_filter, median_filter, sobel_filter};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

pub struct App {
    texture_renderer_base: gbase_utils::TextureRenderer,
    texture_renderer_final: gbase_utils::TextureRenderer,
    framebuffer: render::FrameBuffer,

    texture1: render::GpuImage,
    texture2: render::GpuImage,
    texture3: render::GpuImage,
    texture4: render::GpuImage,
    texture5: render::GpuImage,

    current_texture: ArcTextureView,

    box_filter: box_filter::BoxFilter,
    median_filter: median_filter::MedianFilter,
    sobel_filter: sobel_filter::SobelFilter,
    gaussian_filter: gaussian_filter::GaussianFilter,
}

impl Callbacks for App {
    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.framebuffer.resize(ctx, new_size);
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        // renderers
        let framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
            )
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);
        let texture_renderer_base = gbase_utils::TextureRenderer::new(ctx);
        let texture_renderer_final = gbase_utils::TextureRenderer::new(ctx);

        // textures
        let texture1 = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/nature.jpg").unwrap(),
        )
        .unwrap()
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx)
        .with_default_sampler_and_view(ctx);
        let texture2 = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/city.jpg").unwrap(),
        )
        .unwrap()
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx)
        .with_default_sampler_and_view(ctx);

        let texture3 = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/hellokitty.jpg").unwrap(),
        )
        .unwrap()
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx)
        .with_default_sampler_and_view(ctx);
        let texture4 = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/mario.jpg").unwrap(),
        )
        .unwrap()
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx)
        .with_default_sampler_and_view(ctx);
        let texture5 = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/antialiasing.png").unwrap(),
        )
        .unwrap()
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx)
        .with_default_sampler_and_view(ctx);
        let last_texture = texture1.view();

        // filters
        let box_filter = box_filter::BoxFilter::new(ctx);
        let median_filter = median_filter::MedianFilter::new(ctx);
        let sobel_filter = sobel_filter::SobelFilter::new(ctx);
        let gaussian_filter = gaussian_filter::GaussianFilter::new(ctx);

        Self {
            texture_renderer_final,
            texture_renderer_base,
            framebuffer,

            texture1,
            texture2,
            texture3,
            texture4,
            texture5,
            current_texture: last_texture,

            box_filter,
            median_filter,
            sobel_filter,
            gaussian_filter,
        }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        if input::key_just_pressed(ctx, KeyCode::Backspace)
            || input::key_just_pressed(ctx, KeyCode::KeyR)
        {
            self.texture_renderer_base.render(
                ctx,
                self.current_texture.clone(),
                self.framebuffer.view_ref(),
                wgpu::TextureFormat::Rgba8Unorm,
            );
        }
        if input::key_just_pressed(ctx, KeyCode::Enter)
            || input::key_just_pressed(ctx, KeyCode::Space)
        {
            self.sobel_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &sobel_filter::SobelFilterParams::new(1),
            );
        }

        // textures
        if input::key_just_pressed(ctx, KeyCode::Digit1) {
            self.texture_renderer_base.render(
                ctx,
                self.texture1.view(),
                self.framebuffer.view_ref(),
                wgpu::TextureFormat::Rgba8Unorm,
            );
            self.current_texture = self.texture1.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit2) {
            self.texture_renderer_base.render(
                ctx,
                self.texture2.view(),
                self.framebuffer.view_ref(),
                wgpu::TextureFormat::Rgba8Unorm,
            );
            self.current_texture = self.texture2.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit3) {
            self.texture_renderer_base.render(
                ctx,
                self.texture3.view(),
                self.framebuffer.view_ref(),
                wgpu::TextureFormat::Rgba8Unorm,
            );
            self.current_texture = self.texture3.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit4) {
            self.texture_renderer_base.render(
                ctx,
                self.texture4.view(),
                self.framebuffer.view_ref(),
                wgpu::TextureFormat::Rgba8Unorm,
            );
            self.current_texture = self.texture4.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit5) {
            self.texture_renderer_base.render(
                ctx,
                self.texture5.view(),
                self.framebuffer.view_ref(),
                wgpu::TextureFormat::Rgba8Unorm,
            );
            self.current_texture = self.texture5.view();
        }

        // filters
        if input::key_just_pressed(ctx, KeyCode::F1) {
            self.box_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &box_filter::BoxFilterParams::new(1),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F2) {
            self.box_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &box_filter::BoxFilterParams::new(2),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F3) {
            self.box_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &box_filter::BoxFilterParams::new(3),
            );
        }

        if input::key_just_pressed(ctx, KeyCode::F4) {
            self.median_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &median_filter::MedianFilterParams::new(1),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F5) {
            self.median_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &median_filter::MedianFilterParams::new(2),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F6) {
            self.median_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &median_filter::MedianFilterParams::new(3),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F7) {
            self.gaussian_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &gaussian_filter::GaussianFilterParams::new(1, 1.0),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F8) {
            self.gaussian_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &gaussian_filter::GaussianFilterParams::new(2, 1.5),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F9) {
            self.gaussian_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &gaussian_filter::GaussianFilterParams::new(3, 2.0),
            );
        }

        // final
        self.texture_renderer_final.render(
            ctx,
            self.framebuffer.view(),
            screen_view,
            render::surface_format(ctx),
        );
        false
    }
}
