mod cloud_app;
mod cloud_renderer;
mod noise;

mod triangle;

#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn main() {
    gbase::run::<cloud_app::App>().await;
}
