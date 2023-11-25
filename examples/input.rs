use gbase::{
    input::{self, KeyCode},
    Callbacks, Context,
};

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyA) {
            println!("A pressed")
        }
        if input::key_released(ctx, KeyCode::KeyA) {
            println!("A released")
        }
        println!("{:?}", input::mouse_pos(ctx));
        // println!("{:?}", input::scroll_delta(ctx));
        false
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = gbase::build_context().await;
    let app = App {};
    gbase::run(app, ctx, ev).await;
}
