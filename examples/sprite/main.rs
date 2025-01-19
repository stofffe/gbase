mod sprite_app;
mod sprite_atlas;
mod sprite_renderer;

pub fn main() {
    gbase::run_sync::<sprite_app::App>()
}
