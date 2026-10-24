use gbase::{
    asset::{AssetHandle, SoundLoader, SoundLoaderSettings},
    audio::{self, SoundSource},
    input::{self, KeyCode},
    CallbackResult, Callbacks, Context,
};

pub fn main() {
    gbase::run::<App>();
}

pub struct App {
    sound_handle: AssetHandle<SoundSource>,
}

impl Callbacks for App {
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().assets_path("assets")
    }

    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let sound_handle =
            _cache.load_asset::<SoundLoader>(&SoundLoaderSettings::new("sounds/boom.mp3"));
        Self { sound_handle }
    }

    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        #[cfg(feature = "hot_reload")]
        if gbase::input::key_just_pressed(ctx, gbase::winit::keyboard::KeyCode::F1) {
            gbase::hot_reload::hot_restart(ctx);
        }
        if input::key_just_pressed(ctx, KeyCode::Space) {
            tracing::info!("play boom");
            if let Ok(sound) = _cache.get_asset(&self.sound_handle) {
                audio::play_audio_source(ctx, sound);
            }
        }

        CallbackResult::Continue
    }
}
