use gbase::{
    audio::{self, SoundSource},
    filesystem,
    input::{self, KeyCode},
    Callbacks, Context, ContextBuilder, LogLevel,
};

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    gbase::run::<App>(ctx, ev);
}

struct App {
    sound: SoundSource,
}

impl Callbacks for App {
    fn new(ctx: &mut Context) -> Self {
        let sound_bytes = filesystem::load_b!("sounds/boom.mp3").unwrap();
        let sound = audio::load_audio_source(ctx, sound_bytes);
        Self { sound }
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::Space) {
            log::info!("play boom");
            audio::play_audio_source(ctx, &self.sound);
        }
        false
    }
}
