use gbase::{
    render::{self},
    wgpu,
};

pub struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: render::ArcRenderPipeline,
}

impl gbase::Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut gbase::Context) -> Self {
        println!("INIT");
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            TRIANGLE_VERTICES.to_vec(),
        ))
        .usage(wgpu::BufferUsages::VERTEX)
        .build(ctx);
        let shader_str = include_str!("../assets/shaders/triangle.wgsl").to_string();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new().build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader.clone(), pipeline_layout.clone())
            .buffers(vec![vertex_buffer.desc()])
            .targets(vec![render::RenderPipelineBuilder::default_target(ctx)])
            .build(ctx);

        Self {
            vertex_buffer,
            pipeline,
        }
    }

    #[no_mangle]
    #[allow(unused_variables)]
    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(feature = "hot_reload")]
        {
            if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyH) {
                gbase::hot_reload::hot_reload(ctx);
            }
            if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyR) {
                gbase::hot_reload::hot_restart(ctx);
            }
        }
        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &gbase::wgpu::TextureView) -> bool {
        let mut color;
        color = wgpu::Color::RED;
        color = wgpu::Color::BLUE;

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    // load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })])
            .build_run_new_encoder(ctx, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.draw(0..self.vertex_buffer.len(), 0..1);
            });
        false
    }

    #[no_mangle]
    fn resize(&mut self, _ctx: &mut gbase::Context) {}
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[render::Vertex] = &[
    render::Vertex { position: [-0.5, -0.5, 0.0]  },
    render::Vertex { position: [0.5, -0.5, 0.0]   },
    render::Vertex { position: [0.0, 0.5, 0.0] },
];
