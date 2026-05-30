use gbase::pollster::FutureExt;

fn main() {
    wesl_test::run().block_on();
}
