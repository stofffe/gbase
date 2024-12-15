pub struct App {}

impl App {
    #[no_mangle]
    pub fn new() -> Self {
        Self {}
    }
}

impl gbase::Callbacks for App {
    #[no_mangle]
    fn init(&mut self, _ctx: &mut gbase::Context) {}

    #[no_mangle]
    fn update(&mut self, _ctx: &mut gbase::Context) -> bool {
        println!("yo");
        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &gbase::wgpu::TextureView) -> bool {
        false
    }

    #[no_mangle]
    fn resize(&mut self, _ctx: &mut gbase::Context) {}
}
