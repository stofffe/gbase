use gbase::log;
use gbase::{
    audio::{self, SoundSource},
    filesystem,
    input::{self, KeyCode},
    Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    sound: SoundSource,
}

impl Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let sound_bytes = filesystem::load_b!("sounds/boom.mp3").unwrap();
        let sound = audio::load_audio_source(ctx, sound_bytes);
        Self { sound }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        #[cfg(feature = "hot_reload")]
        if gbase::input::key_just_pressed(ctx, gbase::winit::keyboard::KeyCode::F1) {
            gbase::hot_reload::hot_restart(ctx);
        }
        if input::key_just_pressed(ctx, KeyCode::Space) {
            log::info!("play boom");
            audio::play_audio_source(ctx, &self.sound);
        }
        false
    }
}
