mod app;
mod grass;

#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn main() {
    gbase::run::<app::App>().await
}
