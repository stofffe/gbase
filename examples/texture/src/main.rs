use gbase::pollster::FutureExt;

fn main() {
    texture::run().block_on();
}
