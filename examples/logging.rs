use gbase::{
    input::{self, KeyCode},
    log, Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {}

impl Callbacks for App {
    #[no_mangle]
    fn new(_ctx: &mut Context) -> Self {
        Self {}
    }
    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::Digit1) {
            println!("print");
        }
        if input::key_just_pressed(ctx, KeyCode::Digit2) {
            log::error!("log error");
        }
        if input::key_just_pressed(ctx, KeyCode::Digit3) {
            input::log_error("gbase error");
        }
        false
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, ctx: &mut Context) {
        println!("RELOAD");
        match gbase::env_logger::builder().try_init() {
            Ok(_) => println!("RELOAD OK"),
            Err(_) => println!("RELOAD FAIL"),
        };
    }
}
