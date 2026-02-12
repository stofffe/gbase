use gbase::pollster::FutureExt;

fn main() {
    custom_ui::run().block_on();
}
