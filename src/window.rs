use crate::{
    app::{App, Callbacks},
    Context,
};
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::WindowBuilder,
};

pub(crate) fn new_window() -> (winit::window::Window, winit::event_loop::EventLoop<()>) {
    let event_loop = EventLoop::new().expect("could not initialize event loop");
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("could not initialize window");

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
                // Keyboard
                WindowEvent::KeyboardInput { event, .. } => {
                    let (key, pressed) = (event.physical_key, event.state.is_pressed());
                    match (key, pressed) {
                        (PhysicalKey::Code(code), true) => ctx.input.keyboard.set_key(code),
                        (PhysicalKey::Code(code), false) => ctx.input.keyboard.release_key(code),
                        (PhysicalKey::Unidentified(code), _) => {
                            log::error!("pressed/release  unidentified key {:?}", code)
                        }
                    };
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    ctx.input.keyboard.modifiers_changed(modifiers)
                }
                // Mouse
                WindowEvent::MouseInput { state, button, .. } => {
                    match state {
                        winit::event::ElementState::Pressed => {
                            ctx.input.mouse.press_button(*button)
                        }
                        winit::event::ElementState::Released => {
                            ctx.input.mouse.release_button(*button)
                        }
                    };
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        ctx.input.mouse.set_scroll_delta((*x as f64, *y as f64));
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        ctx.input.mouse.set_scroll_delta((*pos).into());
                    }
                },
                WindowEvent::CursorMoved { position, .. } => {
                    ctx.input.mouse.set_pos((*position).into());
                }
                WindowEvent::CursorEntered { .. } => {
                    ctx.input.mouse.set_on_screen(true);
                }
                WindowEvent::CursorLeft { .. } => {
                    ctx.input.mouse.set_on_screen(false);
                }
                _ => {}
            };
        }
        Event::DeviceEvent { ref event, .. } => {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    ctx.input.mouse.set_mouse_delta(*delta);
                }
                _ => {}
            };
        }
        Event::AboutToWait => {
            app.update(&mut ctx);
        }
        _ => {}
    });
}
