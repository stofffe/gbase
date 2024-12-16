use gbase::Callbacks;

#[pollster::main]
async fn main() {
    let (ctx, ev) = gbase::ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
        .build()
        .await;

    gbase::run::<hot_reload::App>(ctx, ev);
}
