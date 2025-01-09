use gbase::{
    input::{self, KeyCode},
    render::{self, ArcTextureView},
    Callbacks, Context,
};

fn main() {
    gbase::run_sync::<App>();
}

struct App {
    texture_renderer_base: render::TextureRenderer,
    texture_renderer_final: render::TextureRenderer,
    framebuffer: render::FrameBuffer,

    texture1: render::Texture,
    texture2: render::Texture,
    texture3: render::Texture,
    texture4: render::Texture,
    texture5: render::Texture,

    current_texture: ArcTextureView,

    box_filter: render::BoxFilter,
    median_filter: render::MedianFilter,
    sobel_filter: render::SobelFilter,
    gaussian_filter: render::GaussianFilter,
}

impl Callbacks for App {
    fn resize(&mut self, ctx: &mut Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.framebuffer
            .resize(ctx, new_size.width, new_size.height);
    }
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
        let texture_renderer_base =
            render::TextureRenderer::new(ctx, wgpu::TextureFormat::Rgba8Unorm);
        let texture_renderer_final =
            render::TextureRenderer::new(ctx, wgpu::TextureFormat::Bgra8UnormSrgb);

        // textures
        let texture1 = render::TextureBuilder::new(render::TextureSource::Bytes(
            include_bytes!("../assets/textures/nature.jpg").to_vec(),
        ))
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
        let texture2 = render::TextureBuilder::new(render::TextureSource::Bytes(
            include_bytes!("../assets/textures/city.jpg").to_vec(),
        ))
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
        let texture3 = render::TextureBuilder::new(render::TextureSource::Bytes(
            include_bytes!("../assets/textures/hellokitty.jpg").to_vec(),
        ))
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
        let texture4 = render::TextureBuilder::new(render::TextureSource::Bytes(
            include_bytes!("../assets/textures/mario.jpg").to_vec(),
        ))
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
        let texture5 = render::TextureBuilder::new(render::TextureSource::Bytes(
            include_bytes!("../assets/textures/antialiasing.png").to_vec(),
        ))
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
        let last_texture = texture1.view();

        // filters
        let box_filter = render::BoxFilter::new(ctx);
        let median_filter = render::MedianFilter::new(ctx);
        let sobel_filter = render::SobelFilter::new(ctx);
        let gaussian_filter = render::GaussianFilter::new(ctx);

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

    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        if input::key_just_pressed(ctx, KeyCode::Backspace)
            || input::key_just_pressed(ctx, KeyCode::KeyR)
        {
            self.texture_renderer_base.render(
                ctx,
                self.current_texture.clone(),
                self.framebuffer.view_ref(),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::Enter)
            || input::key_just_pressed(ctx, KeyCode::Space)
        {
            self.sobel_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::SobelFilterParams::new(1),
            );
        }

        // textures
        if input::key_just_pressed(ctx, KeyCode::Digit1) {
            self.texture_renderer_base.render(
                ctx,
                self.texture1.view(),
                self.framebuffer.view_ref(),
            );
            self.current_texture = self.texture1.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit2) {
            self.texture_renderer_base.render(
                ctx,
                self.texture2.view(),
                self.framebuffer.view_ref(),
            );
            self.current_texture = self.texture2.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit3) {
            self.texture_renderer_base.render(
                ctx,
                self.texture3.view(),
                self.framebuffer.view_ref(),
            );
            self.current_texture = self.texture3.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit4) {
            self.texture_renderer_base.render(
                ctx,
                self.texture4.view(),
                self.framebuffer.view_ref(),
            );
            self.current_texture = self.texture4.view();
        }
        if input::key_just_pressed(ctx, KeyCode::Digit5) {
            self.texture_renderer_base.render(
                ctx,
                self.texture5.view(),
                self.framebuffer.view_ref(),
            );
            self.current_texture = self.texture5.view();
        }

        // filters
        if input::key_just_pressed(ctx, KeyCode::F1) {
            self.box_filter
                .apply_filter(ctx, &self.framebuffer, &render::BoxFilterParams::new(1));
        }
        if input::key_just_pressed(ctx, KeyCode::F2) {
            self.box_filter
                .apply_filter(ctx, &self.framebuffer, &render::BoxFilterParams::new(2));
        }
        if input::key_just_pressed(ctx, KeyCode::F3) {
            self.box_filter
                .apply_filter(ctx, &self.framebuffer, &render::BoxFilterParams::new(3));
        }

        if input::key_just_pressed(ctx, KeyCode::F4) {
            self.median_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::MedianFilterParams::new(1),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F5) {
            self.median_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::MedianFilterParams::new(2),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F6) {
            self.median_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::MedianFilterParams::new(3),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F7) {
            self.gaussian_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::GaussianFilterParams::new(1, 1.0),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F8) {
            self.gaussian_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::GaussianFilterParams::new(2, 1.5),
            );
        }
        if input::key_just_pressed(ctx, KeyCode::F9) {
            self.gaussian_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::GaussianFilterParams::new(3, 2.0),
            );
        }
        // final
        self.texture_renderer_final
            .render(ctx, self.framebuffer.view(), screen_view);
        false
    }
}
