use egui_wgpu::ScreenDescriptor;
use gbase::{render, tracing, wgpu, winit, Callbacks, Context};

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
    fn render_egui(&mut self, ui: &egui::Context) {
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
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
