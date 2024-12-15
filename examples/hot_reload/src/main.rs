use gbase::Callbacks;

#[pollster::main]
async fn main() {
    let (mut ctx, ev) = gbase::ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
        .build()
        .await;

    let app = hot_reload::App::new(&mut ctx);

    gbase::run(app, ctx, ev);
}
