use gbase::{asset::AssetCache, egui_ui, tracing, CallbackResult, Callbacks, Context};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    name: String,
    age: u32,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().log_level(tracing::Level::ERROR)
    }
    #[no_mangle]
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        Self {
            name: String::new(),
            age: 0,
        }
    }

    #[no_mangle]
    fn render_egui(
        &mut self,
        _ctx: &mut Context,
        _cache: &mut AssetCache,
        egui_ctx: &mut gbase::egui_ui::EguiContext,
    ) -> CallbackResult {
        let mut callback_result = CallbackResult::Continue;

        gbase::egui::Window::new("Stats").show(egui_ctx.ctx(), |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(gbase::egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));

            ui.image(gbase::egui::include_image!(
                "../assets/textures/perlin_noise.png"
            ));

            if ui.button("Exit").clicked() {
                callback_result = CallbackResult::Exit;
            }
        });

        callback_result
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
