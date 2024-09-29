use gbase::{
    filesystem,
    input::{self, KeyCode},
    render, Callbacks, Context,
};

#[pollster::main]
async fn main() {
    let (mut ctx, ev) = gbase::ContextBuilder::new()
        .log_level(gbase::LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    texture_renderer_base: render::TextureRenderer,
    texture_renderer_final: render::TextureRenderer,

    base_texture: render::Texture,
    framebuffer: render::FrameBuffer,
    box_filter: render::BoxFilter,
    median_filter: render::MedianFilter,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        // base
        let texture_bytes = filesystem::load_bytes(ctx, "hellokitty.jpg").await.unwrap();
        let base_texture = render::TextureBuilder::new(render::TextureSource::Bytes(texture_bytes))
            .format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .build(ctx);
        let texture_renderer_base =
            render::TextureRenderer::new(ctx, wgpu::TextureFormat::Rgba8Unorm).await;
        // final
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
        let texture_renderer_final =
            render::TextureRenderer::new(ctx, wgpu::TextureFormat::Bgra8UnormSrgb).await;

        let box_filter = render::BoxFilter::new(ctx).await;
        let median_filter = render::MedianFilter::new(ctx).await;

        Self {
            texture_renderer_final,
            texture_renderer_base,
            framebuffer,
            base_texture,
            box_filter,
            median_filter,
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.texture_renderer_base.render(
                ctx,
                self.base_texture.view(),
                self.framebuffer.view_ref(),
            );
        }
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
        false
    }
    fn init(&mut self, ctx: &mut Context) {
        self.texture_renderer_base.render(
            ctx,
            self.base_texture.view(),
            self.framebuffer.view_ref(),
        );
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        // final
        self.texture_renderer_final
            .render(ctx, self.framebuffer.view(), screen_view);
        false
    }
}
