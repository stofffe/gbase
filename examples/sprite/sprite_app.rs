use gbase::glam::{vec2, Vec2, Vec4};
use gbase::{
    collision::{self, Quad},
    filesystem,
    input::{self, KeyCode},
    render::{self, CameraUniform, VertexTrait},
    time, wgpu, Callbacks, Context,
};

const MAX_SPRITES: u64 = 1000;

pub struct App {
    player: Player,
    obstacles: Vec<Obstacle>,

    camera: render::Camera,
    camera_buffer: render::UniformBuffer<CameraUniform>,

    sprite_renderer: SpriteRenderer,
}

struct Player {
    pos: Vec2,
    size: Vec2,
    _velocity: Vec2,
}

struct Obstacle {
    pos: Vec2,
    size: Vec2,
}

impl Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut gbase::Context) -> Self {
        let player = Player {
            pos: vec2(0.0, 0.0),
            size: vec2(0.1, 0.1),
            _velocity: vec2(0.1, 0.0),
        };
        let obstacles = vec![
            Obstacle {
                pos: vec2(0.3, 0.3),
                size: vec2(0.3, 0.1),
            },
            Obstacle {
                pos: vec2(0.5, 0.5),
                size: vec2(0.2, 0.1),
            },
        ];

        let sprite_renderer =
            SpriteRenderer::new(ctx, MAX_SPRITES, render::surface_config(ctx).format);

        let mut camera = render::Camera::new(render::CameraProjection::orthographic(2.0, 2.0));
        camera.pos.z = 1.0;

        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        Self {
            player,
            obstacles,

            camera,
            camera_buffer,

            sprite_renderer,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(feature = "hot_reload")]
        if input::key_just_pressed(ctx, KeyCode::F1) {
            gbase::hot_reload::hot_restart(ctx);
            println!("hot restart");
        }
        // self.camera.flying_controls(ctx);

        // hot restart
        let dt = time::delta_time(ctx);

        let mut dir = Vec2::ZERO;
        if input::key_pressed(ctx, KeyCode::ArrowUp) {
            dir.y -= 1.0;
        }
        if input::key_pressed(ctx, KeyCode::ArrowDown) {
            dir.y += 1.0;
        }
        if input::key_pressed(ctx, KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if input::key_pressed(ctx, KeyCode::ArrowRight) {
            dir.x += 1.0;
        }

        if dir != Vec2::ZERO {
            self.player.pos += dir.normalize() * dt;
        }

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        // player
        let player_quad = Quad::new(self.player.pos, self.player.size);

        // obstacles
        for obstacle in self.obstacles.iter() {
            let obstacle_quad = Quad::new(obstacle.pos, obstacle.size);
            let color = if collision::quad_quad_collision(player_quad, obstacle_quad) {
                render::RED
            } else {
                render::GREEN
            };
            self.sprite_renderer.draw_quad(obstacle_quad, color);
        }

        self.sprite_renderer.draw_quad(player_quad, render::BLUE);
        self.sprite_renderer
            .render(ctx, screen_view, &self.camera_buffer);

        false
    }
}

struct SpriteRenderer {
    vertices: Vec<VertexSprite>,
    indices: Vec<u32>,

    vertex_buffer: render::VertexBuffer<VertexSprite>,
    index_buffer: render::IndexBuffer,

    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline: render::ArcRenderPipeline,
}

impl SpriteRenderer {
    fn new(ctx: &mut Context, max_sprites: u64, output_format: wgpu::TextureFormat) -> Self {
        let vertices = Vec::new();
        let indices = Vec::new();

        let vertex_buffer =
            render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(max_sprites * 4))
                .build(ctx);
        let index_buffer =
            render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(max_sprites * 6))
                .build(ctx);

        let shader = render::ShaderBuilder::new(
            filesystem::load_s!("shaders/sprite_renderer.wgsl")
                .expect("could not load sprite renderer shader"),
        )
        .build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertex_buffer.desc()])
            .single_target(
                render::ColorTargetState::new()
                    .format(output_format)
                    .blend(wgpu::BlendState::ALPHA_BLENDING),
            )
            .build(ctx);

        Self {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
            bindgroup_layout,
            pipeline,
        }
    }

    fn render(
        &mut self,
        ctx: &mut Context,
        output_view: &wgpu::TextureView,
        camera: &render::UniformBuffer<CameraUniform>,
    ) {
        // update buffers
        self.vertex_buffer.write(ctx, &self.vertices);
        self.index_buffer.write(ctx, &self.indices);

        // create bindgroup
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
            ])
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(output_view).clear(wgpu::Color::BLACK),
            )])
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, bindgroup.as_ref(), &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // TODO: use len of vertices?
                pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());

                pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            });

        render::queue(ctx).submit(Some(encoder.finish()));

        self.vertices.clear();
        self.indices.clear();
    }

    #[rustfmt::skip]
    pub fn draw_quad(&mut self, quad: Quad, color: Vec4) {
        let (x, y) = (quad.pos.x ,quad.pos.y);
        let (sx, sy) = (quad.size.x, quad.size.y);
        let color = color.to_array();

        let offset = self.vertices.len() as u32; // save before pushing verts
        self.vertices.push(VertexSprite { position: [-1.0 + x * 2.0,            1.0 - y * 2.0,            0.0], uv: [0.0, 0.0], color }); // tl
        self.vertices.push(VertexSprite { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0,            0.0], uv: [1.0, 0.0], color }); // tr
        self.vertices.push(VertexSprite { position: [-1.0 + x * 2.0,            1.0 - y * 2.0 - sy * 2.0, 0.0], uv: [0.0, 1.0], color }); // bl
        self.vertices.push(VertexSprite { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0 - sy * 2.0, 0.0], uv: [1.0, 1.0], color }); // br
        self.indices.push(offset);     // tl
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 3); // br
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexSprite {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl VertexSprite {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,   // pos
        1=>Float32x4,   // color
        2=>Float32x2,   // uv
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexSprite {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}
