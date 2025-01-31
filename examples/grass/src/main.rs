use gbase::pollster::FutureExt;

pub fn main() {
    grass::run().block_on();
}
