use gbase::{
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
    gbase::run_manually::<App>(ctx, ev);
}

struct App {}

impl Callbacks for App {
    fn new(_ctx: &mut Context) -> Self {
        let s = filesystem::load_s!("other/test.txt");
        log::warn!("s {:?}", s);

        Self {}
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            let s = filesystem::load_s!("other/test.txt");
            log::warn!("s {:?}", s);
        }
        false
    }
}
