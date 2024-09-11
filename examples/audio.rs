use gbase::{
    audio::{self, SoundSource},
    input::{self, KeyCode},
    Callbacks, Context, ContextBuilder, LogLevel,
};

struct App {
    sound: SoundSource,
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::Space) {
            log::info!("play boom");
            audio::play_audio_source(ctx, &self.sound);
        }
        false
    }
}

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let sound = audio::load_audio_source(&mut ctx, "boom.mp3").await;
    let app = App { sound };
    gbase::run(app, ctx, ev);
}
