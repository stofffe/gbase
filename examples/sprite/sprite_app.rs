use gbase::glam::{vec2, Vec2};
use gbase::render::{surface_config, Transform};
use gbase::{
    collision::{self, Quad},
    input::{self, KeyCode},
    render::{self, CameraUniform},
    time, wgpu, Callbacks,
};
use glam::{vec3, Vec3};

use crate::sprite_atlas::{BACKGROUND, BIRD_FLAP_0, PIPE};
use crate::{sprite_atlas, sprite_renderer};

const MAX_SPRITES: u64 = 1000;

struct Player {
    pos: Vec2,
    velocity: Vec2,
}

struct Obstacle {
    pos: Vec2,
}

pub struct App {
    player: Player,
    obstacles: Vec<Obstacle>,

    camera: render::Camera,
    camera_buffer: render::UniformBuffer<CameraUniform>,

    sprite_renderer: sprite_renderer::SpriteRenderer,

    sprite_atlas: render::TextureWithView,
}

// TODO: add transform to sprite renderer
// scale to flip, rotate

// TODO: look into grid size of 16x16 = 1m?
// now 1px = 1m?

impl Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut gbase::Context) -> Self {
        let player = Player {
            pos: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
        };
        let obstacles = vec![
            Obstacle {
                pos: vec2(0.0, -128.0),
            },
            Obstacle {
                pos: vec2(0.0, 128.0),
            },
        ];

        let sprite_renderer = sprite_renderer::SpriteRenderer::new(
            ctx,
            MAX_SPRITES,
            render::surface_config(ctx).format,
        );

        let mut camera =
            render::Camera::new(render::CameraProjection::orthographic(BACKGROUND.size().y));
        camera.pos.z = 1.0;

        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

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
        self.player.velocity.y -= 9.82 * dt * 80.0;
        self.player.pos += self.player.velocity * dt;

        if input::key_just_pressed(ctx, KeyCode::Space) {
            self.player.velocity.y = 200.0;
        }

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        self.sprite_renderer.draw_sprite(
            Quad::new(-BACKGROUND.size() / 2.0, BACKGROUND.size()),
            BACKGROUND.uv(),
        );

        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        // player
        let player_quad = Quad::new(
            self.player.pos - BIRD_FLAP_0.size() / 2.0,
            BIRD_FLAP_0.size(),
        );

        // obstacles
        for obstacle in self.obstacles.iter() {
            let obstacle_quad = Quad::new(obstacle.pos - PIPE.size() / 2.0, PIPE.size());

            let color = if collision::quad_quad_collision(player_quad, obstacle_quad) {
                render::RED
            } else {
                render::WHITE
            };
            self.sprite_renderer.draw_sprite_with_tint(
                obstacle_quad,
                sprite_atlas::PIPE.uv(),
                color,
            );
        }

        self.sprite_renderer
            .draw_sprite(player_quad, sprite_atlas::BIRD_FLAP_0.uv());

        self.sprite_renderer
            .render(ctx, screen_view, &self.camera_buffer, &self.sprite_atlas);

        // debug
        if input::key_pressed(ctx, KeyCode::F1) {
            // render::TextureRenderer::new(ctx, render::surface_config(ctx).format).render(
            //     ctx,
            //     self.sprite_atlas.view(),
            //     screen_view,
            // );
            let mut gr =
                render::GizmoRenderer::new(ctx, surface_config(ctx).format, &self.camera_buffer);
            gr.draw_cube(&Transform::from_scale(Vec3::ONE * 5.0), Vec3::ONE);
            gr.render(ctx, screen_view);
        }

        false
    }
}
