use gbase::{
    input::{self, KeyCode},
    CallbackResult, Callbacks, Context,
};

pub fn main() {
    gbase::run::<App>();
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
    ) -> CallbackResult {
        let str_path = "tmp/string";
        if input::key_just_pressed(ctx, KeyCode::Digit1) {
            println!("write string");
            // TODO: need async support
            // tracing::warn!("{:?}", filesystem::load_data_string(ctx, str_path));
        }
        if input::key_just_pressed(ctx, KeyCode::Digit2) {
            println!("load string");
            // TODO: need async support
            // filesystem::write_data_string(ctx, str_path, "hello").unwrap();
        }

        let bytes_path = "tmp/bytes";
        if input::key_just_pressed(ctx, KeyCode::Digit3) {
            println!("write bytes");
            // TODO: need async support
            // tracing::warn!("{:?}", filesystem::load_data_bytes(ctx, bytes_path));
        }
        if input::key_just_pressed(ctx, KeyCode::Digit4) {
            println!("load bytes");
            // TODO: need async support
            // filesystem::write_data_bytes(ctx, bytes_path, &[1, 2, 3]).unwrap();
        }

        CallbackResult::Continue
    }
}
