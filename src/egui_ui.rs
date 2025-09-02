use crate::render;

pub(crate) struct EguiContext {
    pub(crate) context: egui::Context,
    pub(crate) state: egui_winit::State,
    pub(crate) renderer: egui_wgpu::Renderer,
}

impl EguiContext {
    pub(crate) fn new(render: &render::RenderContext) -> Self {
        let context = egui::Context::default();
        egui_extras::install_image_loaders(&context); // TODO: have this here?

        let state = egui_winit::State::new(
            context.clone(),
            context.viewport_id(),
            &render.window,
            Some(render.window.scale_factor() as f32),
            None,
            None,
        );
        let renderer =
            egui_wgpu::Renderer::new(&render.device, render.surface_config.format, None, 1, false);

        Self {
            context,
            state,
            renderer,
        }
    }

    pub fn window_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) {
        let _response = self.state.on_window_event(window, event);
    }

    pub fn render(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_size: winit::dpi::PhysicalSize<u32>,
        screen_view: &wgpu::TextureView,
        mut callback: impl FnMut(&egui::Context),
    ) {
        // TODO: on_mouse_motion also?

        let input = self.state.take_egui_input(window);
        let output = self.context.run(input, |ui| callback(ui));

        self.state
            .handle_platform_output(window, output.platform_output);

        let tris = self
            .context
            .tessellate(output.shapes, window.scale_factor() as f32);

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // let mut encoder = render::EncoderBuilder::new().build(ctx);
        let mut encoder =
            device.create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor { label: None });
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: surface_size.into(),
            pixels_per_point: window.scale_factor() as f32,
        };
        self.renderer
            .update_buffers(device, queue, &mut encoder, &tris, &screen_descriptor);

        let mut rpass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui main render pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        drop(rpass);

        queue.submit(Some(encoder.finish()));

        for id in &output.textures_delta.free {
            self.renderer.free_texture(id)
        }
    }
}
