use gbase::pollster::FutureExt;

fn main() {
    physics::run().block_on();
}
