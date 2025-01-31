use gbase::pollster::FutureExt;

fn main() {
    flappy_bird::run().block_on();
}
