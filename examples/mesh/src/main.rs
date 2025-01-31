use gbase::pollster::FutureExt;

fn main() {
    mesh::run().block_on();
}
