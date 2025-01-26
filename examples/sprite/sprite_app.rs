use core::f32;
use gbase::filesystem;
use gbase::glam::vec3;
use gbase::{
    collision::{self, Quad},
    glam::{vec2, Quat, Vec2, Vec3, Vec4Swizzles},
    input::{self, KeyCode},
    render::{self, surface_config, CameraUniform, Transform, Widget, WHITE},
    time, wgpu,
    winit::{dpi::PhysicalSize, window::WindowBuilder},
    Callbacks,
};
use std::f32::consts::PI;

use crate::sprite_atlas::{AtlasSprite, BACKGROUND, BASE, BIRD_FLAP_0, PIPE};
use crate::{sprite_atlas, sprite_renderer};

const MAX_SPRITES: u64 = 1000;

struct Player {
    pos: Vec2,
    velocity: Vec2,
}

impl Player {
    fn new() -> Self {
        Self {
            pos: vec2(-BIRD_FLAP_0.size().x, 0.0),
            velocity: vec2(0.0, 0.0),
        }
    }
}

impl Player {
    fn quad(&self) -> Quad {
        Quad::new(self.pos - BIRD_FLAP_0.size() / 2.0, BIRD_FLAP_0.size())
    }
}

struct PipePair {
    x: f32,
    mid: f32,
    gap: f32,
    collided: bool,
}

impl PipePair {
    fn new() -> Self {
        Self {
            x: BACKGROUND.w as f32 * 1.5,
            mid: rand_range(-PIPE_MAX_OFFSET, PIPE_MAX_OFFSET),
            gap: PIPE_GAP,
            collided: false,
        }
    }
    fn randomize_mid(&mut self) {
        self.mid = rand_range(-PIPE_MAX_OFFSET, PIPE_MAX_OFFSET) + PIPE_BASE_OFFSET;
    }

    fn check_gap_collision(&mut self, player: Quad) -> bool {
        if collision::quad_quad_collision(player, self.gap_quad()) && !self.collided {
            self.collided = true;
            return true;
        }
        false
    }
    fn check_top_bottom_collision(&mut self, player: Quad) -> bool {
        collision::quad_quad_collision(player, self.top_quad())
            || collision::quad_quad_collision(player, self.bottom_quad())
    }

    fn top_pos(&self) -> Vec2 {
        vec2(self.x, PIPE.size().y / 2.0 + self.gap / 2.0 + self.mid)
    }
    fn bottom_pos(&self) -> Vec2 {
        vec2(self.x, -PIPE.size().y / 2.0 - self.gap / 2.0 + self.mid)
    }
    fn gap_pos(&self) -> Vec2 {
        vec2(self.x, self.mid)
    }

    fn top_quad(&self) -> Quad {
        Quad::new(self.top_pos() - PIPE.size() / 2.0, PIPE.size())
    }
    fn bottom_quad(&self) -> Quad {
        Quad::new(self.bottom_pos() - PIPE.size() / 2.0, PIPE.size())
    }
    fn gap_quad(&self) -> Quad {
        Quad::new(
            self.gap_pos() - vec2(PIPE.size().x / 2.0, 0.0),
            vec2(PIPE.size().x, self.gap),
        )
    }
}

struct Base {
    pos: Vec2,
}

impl Base {
    fn quad(&self) -> Quad {
        Quad::new(self.pos, BASE.size())
    }
}

enum GameState {
    StartMenu,
    Game,
    GameOver,
}

pub struct App {
    state: GameState,
    score: u32,

    player: Player,
    pipes: PipePair,
    bases: Vec<Base>,

    camera: render::Camera,
    camera_buffer: render::UniformBuffer<CameraUniform>,

    sprite_renderer: sprite_renderer::SpriteRenderer,

    sprite_atlas: render::TextureWithView,

    ui_renderer: render::GUIRenderer,
}

// TODO: add transform to sprite renderer
// scale to flip, rotate

// TODO: look into grid size of 16x16 = 1m?
// now 1px = 1m?

const PLAYER_FALL_SPEED: f32 = 80.0;
const PLAYER_JUMP_VELOCITY: f32 = 200.0;
const SCROLL_SPEED: f32 = 80.0;
const PIPE_GAP: f32 = 50.0;
const PIPE_MAX_OFFSET: f32 = 50.0;
const PIPE_BASE_OFFSET: f32 = 10.0;

fn rand_range(min: f32, max: f32) -> f32 {
    min + (rand::random::<f32>()) * (max - min)
}
fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    to_min + (to_max - to_min) * ((value - from_min) / (from_max - from_min))
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().window_builder(
            WindowBuilder::new()
                .with_inner_size(PhysicalSize::new(BACKGROUND.w * 4, BACKGROUND.h * 4)),
        )
    }

    #[no_mangle]
    fn new(ctx: &mut gbase::Context) -> Self {
        let player = Player::new();
        let pipes = PipePair::new();
        let bases = vec![
            Base {
                pos: vec2(-BASE.size().x / 2.0, -BACKGROUND.size().y / 2.0),
            },
            Base {
                pos: vec2(
                    -BASE.size().x / 2.0 + BASE.size().x,
                    -BACKGROUND.size().y / 2.0,
                ),
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

        let ui_renderer = render::GUIRenderer::new(
            ctx,
            surface_config(ctx).format,
            1000,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );

        Self {
            state: GameState::StartMenu,
            score: 0,

            player,
            pipes,
            bases,

            camera,
            camera_buffer,

            sprite_renderer,

            sprite_atlas,
            ui_renderer,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(feature = "hot_reload")]
        if input::key_just_pressed(ctx, KeyCode::F1) {
            gbase::hot_reload::hot_restart(ctx);
            println!("hot restart");
        }

        let mut gui_root = Widget::new()
            .width(render::SizeKind::PercentOfParent(1.0))
            .height(render::SizeKind::PercentOfParent(1.0))
            .main_axis_alignment(render::Alignment::Center)
            .cross_axis_alignment(render::Alignment::Center);

        let dt = time::delta_time(ctx);
        match self.state {
            GameState::StartMenu => {
                gui_root.layout(&mut self.ui_renderer, |renderer| {
                    Widget::new()
                        .text("Press Space to Start")
                        .width(render::SizeKind::TextSize)
                        .height(render::SizeKind::TextSize)
                        .text_font_size(50.0)
                        .text_color(WHITE)
                        .render(renderer);
                    Widget::new()
                        .height(render::SizeKind::PercentOfParent(0.3))
                        .render(renderer);
                });

                if input::key_just_pressed(ctx, KeyCode::Space) {
                    self.score = 0;
                    self.state = GameState::Game;
                    self.player.velocity.y = PLAYER_JUMP_VELOCITY;
                }
            }
            GameState::Game => {
                gui_root.layout(&mut self.ui_renderer, |renderer| {
                    Widget::new()
                        .text(self.score.to_string())
                        .width(render::SizeKind::TextSize)
                        .height(render::SizeKind::TextSize)
                        .text_font_size(70.0)
                        .text_color(WHITE)
                        .render(renderer);
                    Widget::new()
                        .height(render::SizeKind::PercentOfParent(0.7))
                        .render(renderer);
                });

                // move player
                self.player.velocity.y -= 9.82 * dt * PLAYER_FALL_SPEED;
                if input::key_just_pressed(ctx, KeyCode::Space) {
                    self.player.velocity.y = PLAYER_JUMP_VELOCITY;
                }
                self.player.pos += self.player.velocity * dt;
                self.player.pos.y = self.player.pos.y.clamp(
                    -BACKGROUND.size().y / 2.0 + BASE.size().y / 2.0,
                    BACKGROUND.size().y / 2.0,
                );

                // move obstacles
                self.pipes.x -= dt * SCROLL_SPEED;
                if self.pipes.x <= -(BACKGROUND.size().x / 2.0 + PIPE.size().x / 2.0) {
                    self.pipes.x += BACKGROUND.size().x + PIPE.size().x;
                    self.pipes.randomize_mid();
                    self.pipes.collided = false;
                }

                // collisions
                if self.pipes.check_top_bottom_collision(self.player.quad()) {
                    self.state = GameState::GameOver;
                }
                if self.pipes.check_gap_collision(self.player.quad()) {
                    self.score += 1;
                }

                // move ground
                for base in self.bases.iter_mut() {
                    base.pos.x -= dt * SCROLL_SPEED;
                    if base.pos.x <= -(BACKGROUND.size().x / 2.0 + BASE.size().x / 2.0) {
                        base.pos.x += BACKGROUND.size().x + BASE.size().x;
                    }
                }
            }
            GameState::GameOver => {
                gui_root
                    .gap(10.0)
                    .layout(&mut self.ui_renderer, |renderer| {
                        Widget::new()
                            .text("Game over")
                            .width(render::SizeKind::TextSize)
                            .height(render::SizeKind::TextSize)
                            .text_font_size(100.0)
                            .text_color(WHITE)
                            .render(renderer);
                        Widget::new()
                            .text(format!("Score: {}", self.score))
                            .width(render::SizeKind::TextSize)
                            .height(render::SizeKind::TextSize)
                            .text_font_size(50.0)
                            .text_color(WHITE)
                            .render(renderer);
                        Widget::new()
                            .text("R to Restart")
                            .width(render::SizeKind::TextSize)
                            .height(render::SizeKind::TextSize)
                            .text_font_size(50.0)
                            .text_color(WHITE)
                            .render(renderer);
                    });

                self.player.velocity.y -= 9.82 * dt * PLAYER_FALL_SPEED;
                self.player.pos += self.player.velocity * dt;
                self.player.pos.y = self.player.pos.y.clamp(
                    -BACKGROUND.size().y / 2.0 + BASE.size().y / 2.0,
                    BACKGROUND.size().y / 2.0,
                );

                if input::key_just_pressed(ctx, KeyCode::KeyR) {
                    self.state = GameState::StartMenu;
                    self.pipes = PipePair::new();
                    self.player = Player::new();
                }
            }
        }

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        // background
        self.sprite_renderer.draw_sprite(
            &Transform::default().with_scale(BACKGROUND.size().extend(1.0)),
            BACKGROUND.uv(),
        );

        // pipes
        self.sprite_renderer.draw_sprite(
            &render::Transform::default()
                .with_pos(self.pipes.top_pos().extend(0.0))
                .with_scale(PIPE.size().extend(1.0) * vec3(1.0, -1.0, 1.0)),
            sprite_atlas::PIPE.uv(),
        );
        self.sprite_renderer.draw_sprite(
            &render::Transform::default()
                .with_pos(self.pipes.bottom_pos().extend(0.0))
                .with_scale(PIPE.size().extend(1.0)),
            sprite_atlas::PIPE.uv(),
        );

        // bases
        for base in self.bases.iter() {
            self.sprite_renderer.draw_sprite(
                &render::Transform::default()
                    .with_pos(base.pos.extend(0.0))
                    .with_scale(BASE.size().extend(1.0)),
                BASE.uv(),
            );
        }

        let player_rot = match self.state {
            GameState::StartMenu => 0.0,
            GameState::Game | GameState::GameOver => {
                remap(self.player.velocity.y, -400.0, 100.0, -PI / 2.0, PI / 4.0)
                    .clamp(-PI / 2.0, PI / 4.0)
            }
        };
        // player
        self.sprite_renderer.draw_sprite(
            &Transform::default()
                .with_pos(self.player.pos.extend(0.0))
                .with_scale(BIRD_FLAP_0.size().extend(1.0))
                .with_rot(Quat::from_rotation_z(player_rot)),
            sprite_atlas::BIRD_FLAP_0.uv(),
        );

        // render to screen
        self.sprite_renderer
            .render(ctx, screen_view, &self.camera_buffer, &self.sprite_atlas);

        self.ui_renderer.render(ctx, screen_view);

        // debug
        if input::key_pressed(ctx, KeyCode::KeyG) {
            // use entities to display outlines
            let mut gr =
                render::GizmoRenderer::new(ctx, surface_config(ctx).format, &self.camera_buffer);
            gr.draw_quad(
                &Transform::from_pos_scale(
                    self.player.pos.extend(0.0),
                    BIRD_FLAP_0.size().extend(0.0),
                ),
                render::RED.xyz(),
            );
            gr.draw_cube(&Transform::from_scale(Vec3::ONE * 5.0), Vec3::ONE);
            gr.render(ctx, screen_view);
        }

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut gbase::Context, new_size: gbase::winit::dpi::PhysicalSize<u32>) {
        self.ui_renderer.resize(ctx, new_size);
    }
}

impl AtlasSprite {
    pub fn size(&self) -> Vec2 {
        vec2(self.w as f32, self.h as f32)
    }
    pub fn uv(&self) -> Quad {
        Quad::new(
            vec2(
                self.x as f32 / sprite_atlas::ATLAS_WIDTH as f32,
                self.y as f32 / sprite_atlas::ATLAS_HEIGHT as f32,
            ),
            vec2(
                self.w as f32 / sprite_atlas::ATLAS_WIDTH as f32,
                self.h as f32 / sprite_atlas::ATLAS_HEIGHT as f32,
            ),
        )
    }
}
