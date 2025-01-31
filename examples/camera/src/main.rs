use gbase::pollster::FutureExt;

fn main() {
    camera::run().block_on();
}
