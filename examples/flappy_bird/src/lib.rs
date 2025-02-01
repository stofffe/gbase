mod sprite_atlas;
mod sprite_renderer;

use crate::sprite_atlas::{AtlasSprite, BACKGROUND, BASE, BIRD_FLAP_0, PIPE};
use core::f32;
use gbase::{
    audio,
    collision::{self, Circle, AABB},
    filesystem,
    glam::{vec2, vec3, Quat, Vec2, Vec3, Vec4Swizzles},
    input::{self, KeyCode},
    load_b, random, render, time, wgpu,
    winit::{dpi::PhysicalSize, window::WindowBuilder},
    Callbacks, Context,
};
use gbase_utils::{Alignment, GammaCorrection, SizeKind, Transform3D, Widget};
use std::f32::consts::PI;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const MAX_SPRITES: u64 = 1000;

struct Player {
    pos: Vec2,
    velocity: Vec2,
    collision_radius: f32,
}

impl Player {
    fn new() -> Self {
        Self {
            pos: vec2(-BIRD_FLAP_0.size().x, 0.0),
            velocity: vec2(0.0, 0.0),
            collision_radius: BIRD_FLAP_0.size().y / 2.0,
        }
    }
}

impl Player {
    fn collision(&self) -> Circle {
        Circle::new(self.pos, self.collision_radius)
    }
}

struct PipePair {
    x: f32,
    mid: f32,
    gap: f32,
    collided: bool,
}

impl PipePair {
    fn new(ctx: &mut Context) -> Self {
        Self {
            x: BACKGROUND.w as f32 * 1.5,
            mid: random::rand(ctx).range_f32(-PIPE_MAX_OFFSET, PIPE_MAX_OFFSET) + PIPE_BASE_OFFSET,
            gap: PIPE_GAP,
            collided: false,
        }
    }
    fn randomize_mid(&mut self, ctx: &mut Context) {
        self.mid =
            random::rand(ctx).range_f32(-PIPE_MAX_OFFSET, PIPE_MAX_OFFSET) + PIPE_BASE_OFFSET;
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

    fn top_collision(&self) -> AABB {
        AABB::new(self.top_pos() - PIPE.size() / 2.0, PIPE.size())
    }
    fn bottom_collision(&self) -> AABB {
        AABB::new(self.bottom_pos() - PIPE.size() / 2.0, PIPE.size())
    }
    fn gap_collision(&self) -> AABB {
        AABB::new(
            self.gap_pos() - vec2(PIPE.size().x / 4.0, 0.0),
            vec2(PIPE.size().x / 2.0, self.gap),
        )
    }
    fn check_gap_collision(&mut self, player: Circle) -> bool {
        if collision::circle_aabb_collision(player, self.gap_collision()) && !self.collided {
            self.collided = true;
            return true;
        }
        false
    }
    fn check_top_bottom_collision(&mut self, player: Circle) -> bool {
        collision::circle_aabb_collision(player, self.top_collision())
            || collision::circle_aabb_collision(player, self.bottom_collision())
    }
}

struct Base {
    base1: Vec2,
    base2: Vec2,
}

impl Base {
    fn new() -> Self {
        Self {
            base1: vec2(-BASE.size().x / 2.0, -BACKGROUND.size().y / 2.0),
            base2: vec2(BASE.size().x / 2.0, -BACKGROUND.size().y / 2.0),
        }
    }
    fn collision(&self) -> AABB {
        AABB::new(
            (self.base1 - BASE.size() / 2.0) * 0.5 + (self.base2 - BASE.size() / 2.0) * 0.5,
            vec2(BASE.size().x * 2.0, BASE.size().y),
        )
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
    bases: Base,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,

    sprite_renderer: sprite_renderer::SpriteRenderer,

    sprite_atlas: render::TextureWithView,

    ui_renderer: gbase_utils::GUIRenderer,

    flap_sound: audio::SoundSource,
    die_sound: audio::SoundSource,
    hit_sound: audio::SoundSource,
    point_sound: audio::SoundSource,

    die_timer: time::Timer,
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
const DIE_TIMER_DURATION: std::time::Duration = std::time::Duration::from_millis(300);

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
        random::seed_with_time(ctx);

        let player = Player::new();
        let pipes = PipePair::new(ctx);
        let bases = Base::new();

        let sprite_renderer =
            sprite_renderer::SpriteRenderer::new(ctx, MAX_SPRITES, render::surface_format(ctx));

        let mut camera = gbase_utils::Camera::new(gbase_utils::CameraProjection::Orthographic {
            height: BACKGROUND.size().y,
        });
        camera.pos.z = 1.0;

        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        let sprite_atlas = gbase_utils::texture_builder_from_image_bytes(sprite_atlas::ATLAS_BYTES)
            .unwrap()
            .format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .build(ctx)
            .with_default_view(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            render::surface_format(ctx),
            1000,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );

        let flap_sound = audio::load_audio_source(ctx, load_b!("sounds/flap.mp3").unwrap());
        let die_sound = audio::load_audio_source(ctx, load_b!("sounds/die.mp3").unwrap());
        let hit_sound = audio::load_audio_source(ctx, load_b!("sounds/hit.mp3").unwrap());
        let point_sound = audio::load_audio_source(ctx, load_b!("sounds/point.mp3").unwrap());

        let die_timer = time::Timer::new(DIE_TIMER_DURATION);

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
            flap_sound,
            die_sound,
            hit_sound,
            point_sound,

            die_timer,
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
            .width(SizeKind::PercentOfParent(1.0))
            .height(SizeKind::PercentOfParent(1.0))
            .main_axis_alignment(Alignment::Center)
            .cross_axis_alignment(Alignment::Center);

        let dt = time::delta_time(ctx);
        match self.state {
            GameState::StartMenu => {
                gui_root.layout(&mut self.ui_renderer, |renderer| {
                    Widget::new()
                        .text("Press Space to Start")
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text_font_size(50.0)
                        .text_color(gbase_utils::WHITE)
                        .render(renderer);
                    Widget::new()
                        .height(SizeKind::PercentOfParent(0.3))
                        .render(renderer);
                });

                if input::key_just_pressed(ctx, KeyCode::Space) {
                    self.score = 0;
                    self.state = GameState::Game;
                    self.player.velocity.y = PLAYER_JUMP_VELOCITY;
                    audio::play_audio_source(ctx, &self.flap_sound);
                }
            }
            GameState::Game => {
                gui_root.layout(&mut self.ui_renderer, |renderer| {
                    Widget::new()
                        .text(self.score.to_string())
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text_font_size(70.0)
                        .text_color(gbase_utils::WHITE)
                        .render(renderer);
                    Widget::new()
                        .height(SizeKind::PercentOfParent(0.7))
                        .render(renderer);
                });

                // move player
                self.player.velocity.y -= 9.82 * dt * PLAYER_FALL_SPEED;
                if input::key_just_pressed(ctx, KeyCode::Space) {
                    self.player.velocity.y = PLAYER_JUMP_VELOCITY;
                    audio::play_audio_source(ctx, &self.flap_sound);
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
                    self.pipes.randomize_mid(ctx);
                    self.pipes.collided = false;
                }

                // move ground
                self.bases.base1.x -= dt * SCROLL_SPEED;
                self.bases.base2.x -= dt * SCROLL_SPEED;
                if self.bases.base1.x <= -(BACKGROUND.size().x / 2.0 + BASE.size().x / 2.0) {
                    self.bases.base1.x += BACKGROUND.size().x + BASE.size().x;
                }
                if self.bases.base2.x <= -(BACKGROUND.size().x / 2.0 + BASE.size().x / 2.0) {
                    self.bases.base2.x += BACKGROUND.size().x + BASE.size().x;
                }

                // score check
                if self.pipes.check_gap_collision(self.player.collision()) {
                    audio::play_audio_source(ctx, &self.point_sound);
                    self.score += 1;
                }

                // game over check
                let mut collided = false;
                collided |= self
                    .pipes
                    .check_top_bottom_collision(self.player.collision());

                if collision::circle_aabb_collision(self.player.collision(), self.bases.collision())
                {
                    collided = true;
                }
                if collided {
                    audio::play_audio_source(ctx, &self.hit_sound);
                    self.state = GameState::GameOver;
                    self.die_timer.reset();
                }
            }
            GameState::GameOver => {
                gui_root
                    .gap(10.0)
                    .layout(&mut self.ui_renderer, |renderer| {
                        Widget::new()
                            .text("Game over")
                            .width(SizeKind::TextSize)
                            .height(SizeKind::TextSize)
                            .text_font_size(100.0)
                            .text_color(gbase_utils::WHITE)
                            .render(renderer);
                        Widget::new()
                            .text(format!("Score: {}", self.score))
                            .width(SizeKind::TextSize)
                            .height(SizeKind::TextSize)
                            .text_font_size(50.0)
                            .text_color(gbase_utils::WHITE)
                            .render(renderer);
                        Widget::new()
                            .text("R to Restart")
                            .width(SizeKind::TextSize)
                            .height(SizeKind::TextSize)
                            .text_font_size(50.0)
                            .text_color(gbase_utils::WHITE)
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
                    self.pipes = PipePair::new(ctx);
                    self.player = Player::new();
                }

                let in_air = !collision::circle_aabb_collision(
                    self.player.collision(),
                    self.bases.collision(),
                );
                if self.die_timer.just_ticked() && in_air {
                    audio::play_audio_source(ctx, &self.die_sound);
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
            &Transform3D::default().with_scale(BACKGROUND.size().extend(1.0)),
            BACKGROUND.uv(),
        );

        // pipes
        self.sprite_renderer.draw_sprite(
            &Transform3D::default()
                .with_pos(self.pipes.top_pos().extend(0.0))
                .with_scale(PIPE.size().extend(1.0) * vec3(1.0, -1.0, 1.0)),
            sprite_atlas::PIPE.uv(),
        );
        self.sprite_renderer.draw_sprite(
            &Transform3D::default()
                .with_pos(self.pipes.bottom_pos().extend(0.0))
                .with_scale(PIPE.size().extend(1.0)),
            sprite_atlas::PIPE.uv(),
        );

        // bases
        self.sprite_renderer.draw_sprite(
            &Transform3D::default()
                .with_pos(self.bases.base1.extend(0.0))
                .with_scale(BASE.size().extend(1.0)),
            BASE.uv(),
        );
        self.sprite_renderer.draw_sprite(
            &Transform3D::default()
                .with_pos(self.bases.base2.extend(0.0))
                .with_scale(BASE.size().extend(1.0)),
            BASE.uv(),
        );

        let player_rot = match self.state {
            GameState::StartMenu => 0.0,
            GameState::Game | GameState::GameOver => {
                remap(self.player.velocity.y, -400.0, 100.0, -PI / 2.0, PI / 4.0)
                    .clamp(-PI / 2.0, PI / 4.0)
            }
        };
        let player_transform = Transform3D::new(
            self.player.pos.extend(0.0),
            Quat::from_rotation_z(player_rot),
            BIRD_FLAP_0.size().extend(1.0),
        );
        // player
        self.sprite_renderer
            .draw_sprite(&player_transform, sprite_atlas::BIRD_FLAP_0.uv());

        // render to screen
        self.sprite_renderer
            .render(ctx, screen_view, &self.camera_buffer, &self.sprite_atlas);

        self.ui_renderer.render(ctx, screen_view);

        // debug
        if input::key_pressed(ctx, KeyCode::KeyG) {
            // use entities to display outlines
            let mut gr = gbase_utils::GizmoRenderer::new(
                ctx,
                render::surface_format(ctx),
                &self.camera_buffer,
            );

            // player
            gr.draw_circle(
                self.player.collision_radius,
                &Transform3D::new(self.player.pos.extend(0.0), Quat::IDENTITY, Vec3::ONE),
                gbase_utils::RED.xyz(),
            );
            // pipes
            gr.draw_quad(
                &Transform3D::from_pos_scale(
                    self.pipes.top_pos().extend(0.0),
                    self.pipes.top_collision().size.extend(1.0),
                ),
                gbase_utils::RED.xyz(),
            );
            gr.draw_quad(
                &Transform3D::from_pos_scale(
                    self.pipes.bottom_pos().extend(0.0),
                    self.pipes.bottom_collision().size.extend(1.0),
                ),
                gbase_utils::RED.xyz(),
            );
            gr.draw_quad(
                &Transform3D::from_pos_scale(
                    self.pipes.gap_pos().extend(1.0),
                    self.pipes.gap_collision().size.extend(1.0),
                ),
                gbase_utils::GREEN.xyz(),
            );

            // base
            gr.draw_quad(
                &Transform3D::from_pos_scale(self.bases.base1.extend(0.0), BASE.size().extend(1.0)),
                gbase_utils::RED.xyz(),
            );
            gr.draw_quad(
                &Transform3D::from_pos_scale(self.bases.base2.extend(0.0), BASE.size().extend(1.0)),
                gbase_utils::RED.xyz(),
            );
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
    pub fn uv(&self) -> AABB {
        AABB::new(
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
