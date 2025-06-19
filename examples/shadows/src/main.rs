use gbase::pollster::FutureExt;

fn main() {
    shadows::run().block_on();
}
