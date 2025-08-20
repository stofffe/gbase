use egui_wgpu::ScreenDescriptor;
use gbase::{render, tracing, wgpu, winit, Callbacks, Context};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct EguiRenderer {
    context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiRenderer {
    pub fn new(ctx: &mut Context) -> Self {
        let window = render::window(ctx);
        let context = egui::Context::default();
        let state = egui_winit::State::new(
            context.clone(),
            context.viewport_id(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let device = render::device(ctx);
        let renderer =
            egui_wgpu::Renderer::new(device, render::surface_format(ctx), None, 1, false);
        Self {
            context,
            state,
            renderer,
        }
    }

    pub fn window_event(&mut self, ctx: &mut Context, event: &winit::event::WindowEvent) {
        let window = render::window(ctx);
        let _response = self.state.on_window_event(window, event);
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        screen_view: &wgpu::TextureView,
        mut callback: impl FnMut(&egui::Context),
    ) {
        // TODO: on_mouse_motion also?

        let window = render::window(ctx);
        let device = render::device(ctx);
        let queue = render::queue(ctx);

        let input = self.state.take_egui_input(window);
        let output = self.context.run(input, |ui| callback(ui));

        self.state
            .handle_platform_output(window, output.platform_output);

        let tris = self
            .context
            .tessellate(output.shapes, window.scale_factor() as f32);

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // let mut encoder = render::EncoderBuilder::new().build(ctx);
        let mut encoder = render::device(ctx)
            .create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor { label: None });
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: render::surface_size(ctx).into(),
            pixels_per_point: window.scale_factor() as f32,
        };
        self.renderer
            .update_buffers(device, queue, &mut encoder, &tris, &screen_descriptor);

        let mut rpass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui main render pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        drop(rpass);

        queue.submit(Some(encoder.finish()));

        for id in &output.textures_delta.free {
            self.renderer.free_texture(id)
        }
    }
}

struct App {
    egui_renderer: EguiRenderer,
    name: String,
    age: u32,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().log_level(tracing::Level::ERROR)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let egui_renderer = EguiRenderer::new(ctx);
        egui_extras::install_image_loaders(&egui_renderer.context);

        Self {
            egui_renderer,
            name: String::new(),
            age: 0,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> bool {
        self.egui_renderer.render(ctx, screen_view, |ui| {
            egui::Window::new("Stats").show(ui, |ui| {
                ui.heading("My egui Application");
                ui.horizontal(|ui| {
                    let name_label = ui.label("Your name: ");
                    ui.text_edit_singleline(&mut self.name)
                        .labelled_by(name_label.id);
                });
                ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
                if ui.button("Increment").clicked() {
                    self.age += 1;
                }
                ui.label(format!("Hello '{}', age {}", self.name, self.age));

                ui.image(egui::include_image!("../assets/textures/perlin_noise.png"));
            });
        });
        false
    }

    fn window_event(&mut self, ctx: &mut Context, event: &gbase::winit::event::WindowEvent) {
        self.egui_renderer.window_event(ctx, event);
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
