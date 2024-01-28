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

pub(crate) fn new_window(
    builder: Option<WindowBuilder>,
) -> (winit::window::Window, winit::event_loop::EventLoop<()>) {
    let event_loop = EventLoop::new().expect("could not initialize event loop");

    let window_builder = builder.unwrap_or_default();

    #[cfg(target_arch = "wasm32")]
    let window_builder = extend_window_builder(window_builder);

    let window = window_builder
        .build(&event_loop)
        .expect("could not initialize window");

    (window, event_loop)
}

#[cfg(target_arch = "wasm32")]
fn extend_window_builder(window_builder: WindowBuilder) -> WindowBuilder {
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowBuilderExtWebSys;

    const WEB_CANVAS_ID: &str = "gbase";

    let win = web_sys::window().expect("could not get window");
    let document = win.document().expect("could not get document");
    let canvas = document
        .get_element_by_id(WEB_CANVAS_ID)
        .expect("could not find canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("element was not a canvas");
    let (width, height) = (canvas.width(), canvas.height());

    // canvas.focus().expect("could not focus canvas");

    window_builder
        .with_canvas(Some(canvas))
        .with_inner_size(winit::dpi::LogicalSize::new(width, height))
}

pub(crate) async fn run_window<C: Callbacks + 'static>(
    event_loop: EventLoop<()>,
    mut app: App<C>,
    mut ctx: Context,
) {
    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run(move |event, target| {
        match event {
            // Update and rendering
            Event::AboutToWait => {
                ctx.render.window().request_redraw();
            }
            // Normal events
            Event::WindowEvent { ref event, .. } => {
                match event {
                    WindowEvent::RedrawRequested => {
                        if app.update(&mut ctx) {
                            target.exit();
                        }
                        if app.render(&mut ctx) {
                            target.exit();
                        }
                    }
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(new_size) => {
                        ctx.render.resize_window(*new_size);
                        app.callbacks.resize(&mut ctx);
                    }
                    // Keyboard
                    WindowEvent::KeyboardInput { event, .. } => {
                        let (key, pressed) = (event.physical_key, event.state.is_pressed());
                        match (key, pressed) {
                            (PhysicalKey::Code(code), true) => ctx.input.keyboard.set_key(code),
                            (PhysicalKey::Code(code), false) => {
                                ctx.input.keyboard.release_key(code)
                            }
                            (PhysicalKey::Unidentified(code), _) => {
                                log::error!("pressed/released unidentified key {:?}", code)
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
            // TODO RedrawRequested and add app.render ?
            _ => {}
        };
    });
}
