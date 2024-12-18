fn main() {
    gbase::ContextBuilder::new().run_sync::<hot_reload::App>();
}
