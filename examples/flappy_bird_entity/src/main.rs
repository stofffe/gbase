use gbase::pollster::FutureExt;

fn main() {
    flappy_bird_entity::run().block_on();
}
