use gbase::pollster::FutureExt;

fn main() {
    ui::run().block_on();
}
