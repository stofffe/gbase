mod grass_app;
mod grass_renderer;

pub fn main() {
    gbase::run_sync::<grass_app::App>()
}
