use gbase::{
    audio::{self, SoundSource},
    filesystem,
    input::{self, KeyCode},
    CallbackResult, Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    sound: SoundSource,
}

impl Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let sound_bytes = filesystem::load_b!("sounds/boom.mp3").unwrap();
        let sound = audio::load_audio_source(ctx, sound_bytes);
        Self { sound }
    }
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        #[cfg(feature = "hot_reload")]
        if gbase::input::key_just_pressed(ctx, gbase::winit::keyboard::KeyCode::F1) {
            gbase::hot_reload::hot_restart(ctx);
        }
        if input::key_just_pressed(ctx, KeyCode::Space) {
            tracing::info!("play boom");
            audio::play_audio_source(ctx, &self.sound);
        }

        CallbackResult::Continue
    }
}
