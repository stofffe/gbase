use gbase::Callbacks;

fn main() {
    gbase::run_app::<hot_reload::App>();
}

// manual

// #[pollster::main]
// async fn main() {
//     let (ctx, ev) = gbase::ContextBuilder::new()
//         .log_level(gbase::LogLevel::Info)
//         .build()
//         .await;
//
//     gbase::run::<hot_reload::App>(ctx, ev);
// }
