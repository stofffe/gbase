use gbase::render::{Widget, BLACK, BLUE, GRAY, GREEN, RED};
use gbase::wgpu;
use gbase::{
    filesystem,
    render::{self},
    Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    gui_renderer: render::GUIRenderer,

    health: f32,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true)
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1000,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );

        Self {
            gui_renderer,
            health: 0.4,
        }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let renderer = &mut self.gui_renderer;

        let outer = Widget::new()
            .label("outer")
            .width(render::SizeKind::PercentOfParent(1.0))
            .height(render::SizeKind::PercentOfParent(1.0))
            .direction(render::Direction::Row)
            .main_axis_alignment(render::Alignment::Center)
            .cross_axis_alignment(render::Alignment::Center)
            .gap(20.0)
            .padding(20.0)
            .color(BLACK)
            .render(ctx, renderer);

        let slider = Widget::new()
            .label("slider")
            .parent(outer)
            .color(GRAY)
            .height(render::SizeKind::Pixels(100.0))
            .width(render::SizeKind::Pixels(500.0))
            .slider(self.health)
            .direction(render::Direction::Row)
            .render(ctx, renderer);
        self.health = slider.slider_value;

        {
            let slider_left = Widget::new()
                .parent(slider)
                .width(render::SizeKind::PercentOfParent(slider.slider_value))
                .render(ctx, renderer);
            let slider_btn = Widget::new()
                .parent(slider)
                .color(GREEN)
                .width(render::SizeKind::Pixels(100.0))
                .height(render::SizeKind::Pixels(100.0))
                .render(ctx, renderer);
        }
        let btn = Widget::new()
            .label("btn")
            .clickable()
            .parent(outer)
            .clickable()
            .color(BLUE)
            .render(ctx, renderer);

        println!("health {}", self.health);

        // let outer = Widget::new()
        //     .label("outer")
        //     .width(render::SizeKind::PercentOfParent(1.0))
        //     .height(render::SizeKind::PercentOfParent(1.0))
        //     .direction(render::Direction::Column)
        //     .gap(20.0)
        //     .padding(20.0)
        //     .color(BLACK)
        //     .render(ctx, renderer);
        // {
        //     let header = Widget::new()
        //         .label("header")
        //         .parent(outer)
        //         .width(render::SizeKind::PercentOfParent(1.0))
        //         .height(render::SizeKind::ChildrenSum)
        //         .direction(render::Direction::Row)
        //         .main_axis_alignment(render::Alignment::Center)
        //         .cross_axis_alignment(render::Alignment::Center)
        //         .gap(20.0)
        //         .padding(20.0)
        //         .color(RED)
        //         .render(ctx, renderer);
        //     {
        //         let header1 = Widget::new()
        //             .label("header1")
        //             .parent(header)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::ChildrenSum)
        //             .text_font_size(80.0)
        //             .padding(10.0)
        //             .gap(10.0)
        //             .color(BLUE)
        //             .render(ctx, renderer);
        //         {
        //             for _ in 0..5 {
        //                 Widget::new()
        //                     .parent(header1)
        //                     .width(render::SizeKind::Grow)
        //                     .height(render::SizeKind::Pixels(40.0))
        //                     .color(GRAY)
        //                     .render(ctx, renderer);
        //             }
        //         }
        //         Widget::new()
        //             .parent(header)
        //             .width(render::SizeKind::Grow)
        //             .render(ctx, renderer);
        //         let header2 = Widget::new()
        //             .label("header2")
        //             .parent(header)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::Pixels(200.0))
        //             .text_font_size(100.0)
        //             .color(BLUE)
        //             .clickable()
        //             .render(ctx, renderer);
        //         let header3 = Widget::new()
        //             .label("header3")
        //             .parent(header)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::Pixels(200.0))
        //             .text_font_size(100.0)
        //             .color(BLUE)
        //             .clickable()
        //             .render(ctx, renderer);
        //     }
        //
        //     let body = Widget::new()
        //         .label("body")
        //         .parent(outer)
        //         .width(render::SizeKind::Grow)
        //         .height(render::SizeKind::Grow)
        //         .direction(render::Direction::Row)
        //         .gap(20.0)
        //         .render(ctx, renderer);
        //
        //     {
        //         let sidebar = Widget::new()
        //             .label("sidebar")
        //             .parent(body)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::Grow)
        //             .color(GRAY)
        //             .render(ctx, renderer);
        //
        //         let content = Widget::new()
        //             .label("content")
        //             .parent(body)
        //             .width(render::SizeKind::Grow)
        //             .height(render::SizeKind::Grow)
        //             .color(GRAY)
        //             .render(ctx, renderer);
        //     }
        //
        //     let footer = Widget::new()
        //         .label("footer")
        //         .parent(outer)
        //         .width(render::SizeKind::Grow)
        //         .height(render::SizeKind::Pixels(200.0))
        //         .color(BLUE)
        //         .render(ctx, renderer);
        // }

        self.gui_renderer.render(ctx, screen_view);
        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context) {
        let new_size = render::surface_size(ctx);
        self.gui_renderer.resize(ctx, new_size);
    }
}
