pub use winit::window::CursorGrabMode;

use crate::{
    app::{App, Callbacks},
    Context,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub(crate) fn new_window() -> (winit::window::Window, winit::event_loop::EventLoop<()>) {
    let event_loop = EventLoop::new().expect("");

    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    attach_window_to_canvas(&window);

    (window, event_loop)
}

#[cfg(target_arch = "wasm32")]
fn attach_window_to_canvas(window: &winit::window::Window) {
    use winit::platform::web::WindowExtWebSys;
    let canvas = window
        .canvas()
        .expect("could not get canvas from winit window");

    let win = web_sys::window().expect("could not get window");
    let document = win.document().expect("could not get document");
    let body = document.body().expect("could not get body");
    body.append_child(&canvas)
        .expect("could not append canvas to body");

    // Auto focus canvas
    canvas.focus().expect("could not focus canvas");
}

pub(crate) async fn run_window<C: Callbacks + 'static>(
    event_loop: EventLoop<()>,
    mut app: App<C>,
    mut ctx: Context,
) {
    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run(move |event, target| match event {
        Event::WindowEvent { ref event, .. } => {
            match event {
                WindowEvent::CloseRequested => target.exit(),
                _ => {}
            };
        }
        Event::AboutToWait => {
            app.update(&mut ctx);
        }
        _ => {}
    });
}
