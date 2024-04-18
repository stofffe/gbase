use gbase::{
    render::{
        self, DynamicIndexBuffer, DynamicIndexBufferBuilder, DynamicVertexBuffer,
        DynamicVertexBufferBuilder, EncoderBuilder, RenderPipelineBuilder, ShaderBuilder,
        VertexColor,
    },
    Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, Vec2, Vec3};
use std::f32::consts::PI;

struct App {
    gizmo_renderer: GizmoRenderer,
}

impl App {
    fn new(ctx: &Context) -> Self {
        let gizmo_renderer = GizmoRenderer::new(ctx);
        Self { gizmo_renderer }
    }
}
const RED: Vec3 = vec3(1.0, 0.0, 0.0);
const GREEN: Vec3 = vec3(0.0, 1.0, 0.0);
const BLUE: Vec3 = vec3(1.0, 0.0, 1.0);
const WHITE: Vec3 = vec3(1.0, 1.0, 1.0);

impl Callbacks for App {
    fn update(&mut self, _ctx: &mut Context) -> bool {
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gizmo_renderer
            .draw_2d_line(vec2(0.0, 0.0), vec2(0.5, 0.5), GREEN);
        self.gizmo_renderer
            .draw_2d_quad(vec2(0.0, 0.0), vec2(0.2, 0.2), RED);
        self.gizmo_renderer
            .draw_2d_circle(vec2(0.0, 0.0), 0.2, BLUE);
        //
        self.gizmo_renderer.render(ctx, screen_view);
        false
    }
    fn resize(&mut self, ctx: &mut Context) {
        self.gizmo_renderer.resize(ctx);
    }
}

struct GizmoRenderer {
    vertex_buffer: DynamicVertexBuffer<VertexColor>,
    index_buffer: DynamicIndexBuffer,
    pipeline: wgpu::RenderPipeline,
    depth_buffer: render::DepthBuffer,
}

const GIZMO_MAX_VERTICES: usize = 10000;
const GIZMO_MAX_INDICES: usize = 10000;
impl GizmoRenderer {
    fn new(ctx: &Context) -> Self {
        let vertex_buffer = DynamicVertexBufferBuilder::new()
            .capacity(GIZMO_MAX_VERTICES)
            .build(ctx);
        let index_buffer = DynamicIndexBufferBuilder::new()
            .capacity(GIZMO_MAX_INDICES)
            .build(ctx);
        let shader = ShaderBuilder::new(include_str!("../assets/gizmo.wgsl")).build(ctx);
        let pipeline = RenderPipelineBuilder::new(&shader)
            .buffers(&[vertex_buffer.desc()])
            .targets(&[RenderPipelineBuilder::default_target(ctx)])
            .depth_stencil(render::DepthBuffer::depth_stencil_state())
            .topology(wgpu::PrimitiveTopology::LineList)
            .build(ctx);

        let depth_buffer = render::DepthBuffer::new(ctx);

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            depth_buffer,
        }
    }
    fn render(&mut self, ctx: &Context, view: &wgpu::TextureView) {
        self.vertex_buffer.update_buffer(ctx);
        self.index_buffer.update_buffer(ctx);

        let mut encoder = EncoderBuilder::new().build(ctx);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_clear()),
            label: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());
        pass.draw_indexed(0..self.index_buffer.len(), 0, 0..1);
        drop(pass);

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));

        self.vertex_buffer.clear();
        self.index_buffer.clear();
    }
    fn resize(&mut self, ctx: &Context) {
        self.depth_buffer.resize(ctx);
    }

    fn draw_2d_line(&mut self, start: Vec2, end: Vec2, color: Vec3) {
        let vertex_start = self.vertex_buffer.len();
        self.vertex_buffer.add(VertexColor {
            position: [start.x, start.y, 0.0],
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: [end.x, end.y, 0.0],
            color: color.to_array(),
        });
        self.index_buffer.add(vertex_start);
        self.index_buffer.add(vertex_start + 1);
    }
    fn draw_2d_quad(&mut self, center: Vec2, dim: Vec2, color: Vec3) {
        let c = center;
        let vertex_start = self.vertex_buffer.len();
        self.vertex_buffer.add(VertexColor {
            position: [c.x - dim.x / 2.0, c.y - dim.y / 2.0, 0.0],
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: [c.x + dim.x / 2.0, c.y - dim.y / 2.0, 0.0],
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: [c.x + dim.x / 2.0, c.y + dim.y / 2.0, 0.0],
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: [c.x - dim.x / 2.0, c.y + dim.y / 2.0, 0.0],
            color: color.to_array(),
        });

        self.index_buffer.add(vertex_start);
        self.index_buffer.add(vertex_start + 1);

        self.index_buffer.add(vertex_start + 1);
        self.index_buffer.add(vertex_start + 2);

        self.index_buffer.add(vertex_start + 2);
        self.index_buffer.add(vertex_start + 3);

        self.index_buffer.add(vertex_start + 3);
        self.index_buffer.add(vertex_start);
    }
    /// Draw a
    fn draw_2d_circle(&mut self, center: Vec2, radius: f32, color: Vec3) {
        const N: usize = 16;

        let vertex_start = self.vertex_buffer.len();

        for i in 0..N {
            let p = i as f32 / N as f32;
            let angle = p * 2.0 * PI;
            self.vertex_buffer.add(VertexColor {
                position: [
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                    0.0,
                ],
                color: color.to_array(),
            });
        }

        for i in 0..(N - 1) as u32 {
            self.index_buffer.add(vertex_start + i);
            self.index_buffer.add(vertex_start + i + 1);
        }
        self.index_buffer.add(vertex_start + N as u32 - 1);
        self.index_buffer.add(vertex_start);
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&ctx);
    gbase::run(app, ctx, ev).await;
}

// fn draw_quad_tl(&mut self, tl: Vec2, dim: Vec2, color: Vec3) {
//         let vertex_start = self.vertex_buffer.len();
//         self.vertex_buffer.add(VertexColor {
//             position: [tl.x, tl.y, 0.0],
//             color: color.to_array(),
//         });
//         self.vertex_buffer.add(VertexColor {
//             position: [tl.x + dim.x, tl.y, 0.0],
//             color: color.to_array(),
//         });
//         self.vertex_buffer.add(VertexColor {
//             position: [tl.x + dim.x, tl.y + dim.y, 0.0],
//             color: color.to_array(),
//         });
//         self.vertex_buffer.add(VertexColor {
//             position: [tl.x, tl.y + dim.y, 0.0],
//             color: color.to_array(),
//         });
//
//         self.index_buffer.add(vertex_start);
//         self.index_buffer.add(vertex_start + 1);
//
//         self.index_buffer.add(vertex_start + 1);
//         self.index_buffer.add(vertex_start + 2);
//
//         self.index_buffer.add(vertex_start + 2);
//         self.index_buffer.add(vertex_start + 3);
//
//         self.index_buffer.add(vertex_start + 3);
//         self.index_buffer.add(vertex_start);
//     }
