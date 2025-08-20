use gbase::pollster::FutureExt;

fn main() {
    egui_ui::run().block_on();
}
