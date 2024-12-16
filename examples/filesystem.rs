use gbase::{
    filesystem,
    input::{self, KeyCode},
    Callbacks, Context, ContextBuilder, LogLevel,
};

pub fn main() {
    gbase::run_app_with_builder::<App>(ContextBuilder::new().log_level(LogLevel::Info));
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
