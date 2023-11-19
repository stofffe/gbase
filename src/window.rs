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

    (window, event_loop)
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
            println!("draw");
        }
        _ => {}
    });
}
