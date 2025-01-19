mod atlas;
mod sprite_app;
mod sprite_packer;

pub fn main() {
    gbase::run_sync::<sprite_app::App>()
}
