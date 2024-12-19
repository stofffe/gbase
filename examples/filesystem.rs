use gbase::{
    filesystem,
    input::{self, KeyCode},
    Callbacks, Context, LogLevel,
};

pub fn main() {
    gbase::run_sync::<App>();
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
