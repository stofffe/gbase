use gbase::pollster::FutureExt;

fn main() {
    shader_imports::run().block_on();
}
