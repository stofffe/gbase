use gbase::pollster::FutureExt;

pub fn main() {
    clouds::run().block_on();
}
