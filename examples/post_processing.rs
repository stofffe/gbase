use gbase::{filesystem, render, time, Callbacks, Context};

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

    // middle_unorm_view: render::ArcTextureView,
    render_unorm_view: render::ArcTextureView,
    base_texture: render::Texture,
    render_texture: render::FrameBuffer,
    middle_texture: render::FrameBuffer,
    box_filter: render::BoxFilter,
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
        // middle
        let middle_texture = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .build(ctx);
        let box_filter = render::BoxFilter::new(ctx).await;
        // final
        let render_texture = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);
        let render_unorm_view = render::ArcTextureView::new(render_texture.texture().create_view(
            &wgpu::TextureViewDescriptor {
                label: Some("render unorm"),
                format: Some(wgpu::TextureFormat::Rgba8Unorm),
                ..Default::default()
            },
        ));
        let texture_renderer_final =
            render::TextureRenderer::new(ctx, wgpu::TextureFormat::Bgra8UnormSrgb).await;

        Self {
            texture_renderer_final,
            texture_renderer_base,
            render_texture,
            box_filter,
            base_texture,
            middle_texture,
            // middle_unorm_view,
            render_unorm_view,
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        log::warn!("fps {}", time::fps(ctx));
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        // base
        self.texture_renderer_base.render(
            ctx,
            self.base_texture.view(),
            self.middle_texture.view_ref(),
        );

        self.box_filter.apply_filter(
            ctx,
            self.middle_texture.view(),
            self.render_unorm_view.clone(),
            self.render_texture.texture().width(),
            self.render_texture.texture().height(),
        );

        // final
        self.texture_renderer_final
            .render(ctx, self.render_texture.view(), screen_view);
        false
    }
}
