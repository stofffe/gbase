use gbase::glam::{vec2, Vec2};
use gbase::{
    collision::{self, Quad},
    input::{self, KeyCode},
    render::{self, CameraUniform},
    time, wgpu, Callbacks,
};

use crate::{sprite_atlas, sprite_renderer};

const MAX_SPRITES: u64 = 1000;

struct Player {
    pos: Vec2,
    size: Vec2,
    _velocity: Vec2,
}

struct Obstacle {
    pos: Vec2,
    size: Vec2,
}

pub struct App {
    player: Player,
    obstacles: Vec<Obstacle>,

    camera: render::Camera,
    camera_buffer: render::UniformBuffer<CameraUniform>,

    sprite_renderer: sprite_renderer::SpriteRenderer,

    atlas_renderer: render::TextureRenderer,

    sprite_atlas: render::TextureWithView,
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

        let sprite_renderer = sprite_renderer::SpriteRenderer::new(
            ctx,
            MAX_SPRITES,
            render::surface_config(ctx).format,
        );

        let mut camera = render::Camera::new(render::CameraProjection::orthographic(2.0, 2.0));
        camera.pos.z = 1.0;

        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        let atlas_renderer = render::TextureRenderer::new(ctx, render::surface_config(ctx).format);

        let sprite_atlas = render::TextureBuilder::new(render::TextureSource::Bytes(
            sprite_atlas::ATLAS_BYTES.to_vec(),
        ))
        .build(ctx)
        .with_default_view(ctx);

        Self {
            player,
            obstacles,

            camera,
            camera_buffer,

            sprite_renderer,

            atlas_renderer,
            sprite_atlas,
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

        // self.sprite_renderer.draw_quad(player_quad, render::BLUE);
        //
        // self.sprite_renderer.draw_quad(player_quad, render::BLUE);

        self.sprite_renderer.draw_sprite(
            player_quad,
            sprite_atlas::BIRD_FLAP_0.into(),
            render::WHITE,
        );

        self.sprite_renderer
            .render(ctx, screen_view, &self.camera_buffer, &self.sprite_atlas);

        // self.atlas_renderer
        //     .render(ctx, self.sprite_atlas.view(), screen_view);

        false
    }
}
