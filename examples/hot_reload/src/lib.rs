pub struct App {}

impl gbase::Callbacks for App {
    #[no_mangle]
    fn new(_ctx: &mut gbase::Context) -> Self {
        Self {}
    }

    #[no_mangle]
    fn update(&mut self, _ctx: &mut gbase::Context) -> bool {
        println!("a");
        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &gbase::wgpu::TextureView) -> bool {
        false
    }

    #[no_mangle]
    fn resize(&mut self, _ctx: &mut gbase::Context) {}
}
