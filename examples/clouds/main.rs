mod cloud_app;
mod cloud_renderer;
mod noise;

pub fn main() {
    gbase::run_sync::<cloud_app::App>()
}
