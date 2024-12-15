#[pollster::main]
async fn main() {
    let app = hot_reload::App::new();

    let (ctx, ev) = gbase::ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
        .build()
        .await;

    gbase::run(app, ctx, ev);
}
