use gbase::{Callbacks, Context};

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        println!("update");
        false
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = gbase::build_context().await;
    let app = App {};
    gbase::run(app, ctx, ev).await;
}

