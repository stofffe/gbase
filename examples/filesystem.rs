use gbase::{
    filesystem,
    input::{self, KeyCode},
    Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

struct App {}

impl Callbacks for App {
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        Self {}
    }

    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> bool {
        let str_path = "tmp/string";
        if input::key_just_pressed(ctx, KeyCode::Digit1) {
            println!("write string");
            tracing::warn!("{:?}", filesystem::load_str_tmp(ctx, str_path));
        }
        if input::key_just_pressed(ctx, KeyCode::Digit2) {
            println!("load string");
            filesystem::store_str_tmp(ctx, str_path, "hello").unwrap();
        }

        let bytes_path = "tmp/bytes";
        if input::key_just_pressed(ctx, KeyCode::Digit3) {
            println!("write bytes");
            tracing::warn!("{:?}", filesystem::load_bytes_tmp(ctx, bytes_path));
        }
        if input::key_just_pressed(ctx, KeyCode::Digit4) {
            println!("load bytes");
            filesystem::store_bytes_tmp(ctx, bytes_path, &[1, 2, 3]).unwrap();
        }

        false
    }
}
