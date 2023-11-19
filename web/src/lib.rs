use gbase::glam;
use gbase::wgpu;
use gbase::winit;
use gbase::log;
use gbase::{Callbacks, Context};

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        log::info!("info");
        log::warn!("warn");
        log::error!("error");
        false
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn run() {
    let (ctx, ev) = gbase::build_context().await;
    log::error!("yoo");
    let app = App {};
    gbase::run(app, ctx, ev).await;
}
