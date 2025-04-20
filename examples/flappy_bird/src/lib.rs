mod sprite_atlas;
mod sprite_renderer;

use crate::sprite_atlas::{AtlasSprite, BACKGROUND};
use core::f32;
use gbase::{
    audio, collision, filesystem,
    glam::{vec2, Quat, Vec2, Vec3, Vec4Swizzles},
    input::{self, KeyCode},
    load_b, random, render, time, wgpu,
    winit::{dpi::PhysicalSize, window::Window},
    Callbacks, Context,
};
use gbase_utils::{Alignment, SizeKind, Transform2D, Transform3D, Widget};
use sprite_atlas::{BASE, BIRD_FLAP_0, BIRD_FLAP_1, PIPE};
use std::{f32::consts::PI, time::Duration};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
struct EntityHandle(usize);

impl EntityHandle {
    const ROOT: Self = Self(0);
    fn index(&self) -> usize {
        self.0
    }
}

impl EntityHandle {
    /// Turn handle into reference
    fn get(self, handler: &EntityHandler) -> &Entity {
        handler.get_entity(self)
    }
    /// Turn handle into mutable reference
    fn get_mut(self, handler: &mut EntityHandler) -> &mut Entity {
        handler.get_entity_mut(self)
    }
}

#[derive(Debug, Default, Clone, Copy)]
enum Collision {
    #[default]
    None,
    Circle {
        radius: f32,
    },
    Quad {
        size: Vec2,
    },
}

#[derive(Debug, Default)]
enum Renderable {
    #[default]
    None,
    Sprite,
    Animation,
}

#[derive(Debug, Default)]
struct Animation {
    timer: time::Timer,
    current: usize,
    sprites: Vec<Sprite>,
}

impl Animation {
    fn new(sprites: Vec<Sprite>, speed: Duration) -> Self {
        Self {
            timer: time::Timer::new(speed),
            sprites,
            current: 0,
        }
    }

    fn tick(&mut self) {
        if self.timer.just_ticked() {
            self.current += 1;
            self.current %= self.sprites.len();
            self.timer.reset();
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Sprite {
    atlas_pos: Vec2,
    atlas_size: Vec2,
}

impl Sprite {
    fn new(atlas_pos: Vec2, atlas_size: Vec2) -> Self {
        Self {
            atlas_pos,
            atlas_size,
        }
    }
}

#[derive(Default)]
struct Entity {
    handle: EntityHandle,
    parent: EntityHandle,

    local_pos: Vec2,
    local_scale: Vec2,
    local_rotation: f32,

    renderable: Renderable,
    sprite: Sprite,
    animation: Animation,

    velocity: Vec2,

    collision: Collision,
    obstacle: bool,
    score_child: EntityHandle,
    score_area: bool,
    is_pipe: bool,
}

impl Entity {
    /// Calculate global pos
    fn pos(&self, handler: &EntityHandler) -> Vec2 {
        handler.calc_pos(self)
    }
}

struct EntityHandler {
    entities: Vec<Entity>,
}

impl EntityHandler {
    fn new() -> Self {
        let root = Entity {
            parent: EntityHandle(0),
            local_pos: Vec2::ZERO,
            ..Default::default()
        };
        let entities = vec![root];
        Self { entities }
    }
    fn create_entity(&mut self, mut entity: Entity) -> EntityHandle {
        let handle = EntityHandle(self.entities.len());
        entity.handle = handle;
        self.entities.push(entity);
        handle
    }
    fn get_entity(&self, entity: EntityHandle) -> &Entity {
        &self.entities[entity.index()]
    }
    fn get_entity_mut(&mut self, entity: EntityHandle) -> &mut Entity {
        &mut self.entities[entity.index()]
    }

    fn get_handles(&self) -> Vec<EntityHandle> {
        self.entities.iter().map(|e| e.handle).collect()
    }
    fn get_handles_filtered(&self, filter_func: fn(&Entity) -> bool) -> Vec<EntityHandle> {
        self.entities
            .iter()
            .filter(|a| filter_func(a))
            .map(|e| e.handle)
            .collect()
    }

    fn calc_pos(&self, entity: &Entity) -> Vec2 {
        let mut e = entity;
        let mut pos = e.local_pos;
        while e.parent != EntityHandle::ROOT {
            e = self.get_entity(e.parent);
            pos += e.local_pos;
        }
        pos
    }

    fn check_collision_handle(&self, e1: EntityHandle, e2: EntityHandle) -> bool {
        let e1 = self.get_entity(e1);
        let e2 = self.get_entity(e2);
        self.check_entity_collision(e1, e2)
    }

    #[rustfmt::skip]
    fn check_entity_collision(&self, e1: &Entity, e2: &Entity) -> bool {
        match e1.collision {
            Collision::None => false,
            Collision::Circle { radius: r1 } => match e2.collision {
                Collision::None => false,
                Collision::Circle { radius: r2 } => collision::circle_circle_collision(
                    collision::Circle { origin: self.calc_pos(e1), radius: r1, },
                    collision::Circle { origin: self.calc_pos(e2), radius: r2, },
                ),
                Collision::Quad { size: s2 } => collision::circle_aabb_collision(
                    collision::Circle { origin: self.calc_pos(e1), radius: r1, },
                    collision::AABB { center: self.calc_pos(e2), size: s2, },
                ),
            },
            Collision::Quad { size: s1 } => match e2.collision {
                Collision::None => false,
                Collision::Circle { radius: r2 } => collision::circle_aabb_collision(
                    collision::Circle { origin: self.calc_pos(e2), radius: r2, },
                    collision::AABB { center: self.calc_pos(e1), size: s1, },
                ),
                Collision::Quad { size: s2 } => collision::aabb_aabb_collision(
                    collision::AABB { center: self.calc_pos(e1), size: s1, },
                    collision::AABB { center: self.calc_pos(e2), size: s2, },
                ),
            },
        }
    }
}

enum GameState {
    StartMenu,
    Game,
    GameOver,
}

pub struct App {
    entities: EntityHandler,

    player: EntityHandle,
    pipe_middle: EntityHandle,
    base_middle: EntityHandle,

    state: GameState,
    score: u32,
    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    sprite_renderer: sprite_renderer::SpriteRenderer,
    sprite_atlas: render::GpuImage,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,
    highscore: u32,
    die_timer: time::Timer,

    flap_sound: audio::SoundSource,
    die_sound: audio::SoundSource,
    hit_sound: audio::SoundSource,
    point_sound: audio::SoundSource,
}

const MAX_SPRITES: u64 = 1000;
const PLAYER_FALL_SPEED: f32 = 80.0;
const PLAYER_JUMP_VELOCITY: f32 = 200.0;
const SCROLL_SPEED: f32 = 80.0;
const PIPE_GAP: f32 = 50.0;
const PIPE_MAX_OFFSET: f32 = 50.0;
const PIPE_BASE_OFFSET: f32 = 10.0;
const DIE_TIMER_DURATION: std::time::Duration = std::time::Duration::from_millis(300);
const HIGHSCORE_PATH: &str = "highscore";
const BIRD_ANIMATION_SPEED: Duration = Duration::from_millis(70);

const STENCIL_SKIP: u32 = 0;
const STENCIL_FULL: u32 = 1;
const STENCIL_OBSTACLES: u32 = 2;

fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    to_min + (to_max - to_min) * ((value - from_min) / (from_max - from_min))
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .window_attributes(
                Window::default_attributes()
                    .with_inner_size(PhysicalSize::new(BACKGROUND.w * 4, BACKGROUND.h * 4)),
            )
            .log_level(gbase::LogLevel::Warn)
    }

    #[no_mangle]
    fn new(ctx: &mut gbase::Context) -> Self {
        random::seed_with_time(ctx);

        let mut entities = EntityHandler::new();

        // entities
        let player = entities.create_entity(Entity {
            local_pos: vec2(-BIRD_FLAP_0.pixel_size().x, 0.0),
            local_scale: BIRD_FLAP_0.pixel_size(),
            renderable: Renderable::Sprite,
            sprite: Sprite::new(BIRD_FLAP_0.atlas_pos(), BIRD_FLAP_0.atlas_size()),
            animation: Animation::new(
                vec![
                    Sprite::new(BIRD_FLAP_0.atlas_pos(), BIRD_FLAP_0.atlas_size()),
                    Sprite::new(BIRD_FLAP_1.atlas_pos(), BIRD_FLAP_1.atlas_size()),
                ],
                BIRD_ANIMATION_SPEED,
            ),
            collision: Collision::Circle {
                radius: BIRD_FLAP_0.pixel_size().y / 2.0,
            },
            ..Default::default()
        });
        // pipes
        let pipe_middle = entities.create_entity(Entity {
            local_pos: vec2(BACKGROUND.w as f32, 0.0),
            ..Default::default()
        });
        let _top_pipe = entities.create_entity(Entity {
            parent: pipe_middle,
            local_pos: vec2(0.0, PIPE.pixel_size().y / 2.0 + PIPE_GAP / 2.0),
            local_scale: PIPE.pixel_size() * vec2(1.0, -1.0),
            sprite: Sprite::new(PIPE.atlas_pos(), PIPE.atlas_size()),
            renderable: Renderable::Sprite,
            collision: Collision::Quad {
                size: PIPE.pixel_size(),
            },
            obstacle: true,
            is_pipe: true,
            ..Default::default()
        });
        let _bottom_pipe = entities.create_entity(Entity {
            parent: pipe_middle,
            local_pos: vec2(0.0, -(PIPE.pixel_size().y / 2.0 + PIPE_GAP / 2.0)),
            local_scale: PIPE.pixel_size(),
            sprite: Sprite::new(PIPE.atlas_pos(), PIPE.atlas_size()),
            renderable: Renderable::Sprite,
            collision: Collision::Quad {
                size: PIPE.pixel_size(),
            },
            obstacle: true,
            is_pipe: true,
            ..Default::default()
        });
        let score_area = entities.create_entity(Entity {
            parent: pipe_middle,
            local_scale: vec2(PIPE.pixel_size().x / 2.0, PIPE_GAP),
            collision: Collision::Quad {
                size: vec2(PIPE.pixel_size().x / 2.0, PIPE_GAP),
            },
            score_area: true,
            ..Default::default()
        });
        entities.get_entity_mut(pipe_middle).score_child = score_area;

        // bases
        let base_middle = entities.create_entity(Entity {
            local_pos: vec2(0.0, -BACKGROUND.pixel_size().y / 2.0),
            collision: Collision::Quad {
                size: vec2(BASE.pixel_size().x * 3.0, BASE.pixel_size().y),
            },
            obstacle: true,
            ..Default::default()
        });
        let _base_1 = entities.create_entity(Entity {
            parent: base_middle,
            local_pos: vec2(-BASE.pixel_size().x, 0.0),
            local_scale: BASE.pixel_size(),
            sprite: Sprite::new(BASE.atlas_pos(), BASE.atlas_size()),
            renderable: Renderable::Sprite,
            ..Default::default()
        });
        let _base_2 = entities.create_entity(Entity {
            parent: base_middle,
            local_pos: vec2(0.0, 0.0),
            local_scale: BASE.pixel_size(),
            sprite: Sprite::new(BASE.atlas_pos(), BASE.atlas_size()),
            renderable: Renderable::Sprite,
            ..Default::default()
        });
        let _base_3 = entities.create_entity(Entity {
            parent: base_middle,
            local_pos: vec2(BASE.pixel_size().x, 0.0),
            local_scale: BASE.pixel_size(),
            sprite: Sprite::new(BASE.atlas_pos(), BASE.atlas_size()),
            renderable: Renderable::Sprite,
            ..Default::default()
        });

        // other
        let sprite_renderer =
            sprite_renderer::SpriteRenderer::new(ctx, MAX_SPRITES, render::surface_format(ctx));
        let mut camera = gbase_utils::Camera::new(gbase_utils::CameraProjection::Orthographic {
            height: BACKGROUND.pixel_size().y,
        });
        camera.pos.z = 1.0;

        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        let sprite_atlas = gbase_utils::texture_builder_from_image_bytes(sprite_atlas::ATLAS_BYTES)
            .unwrap()
            .format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .build(ctx)
            .with_default_sampler_and_view(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1000,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );

        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let highscore = if let Ok(data) = filesystem::load_str(ctx, HIGHSCORE_PATH) {
            data.trim().parse::<u32>().unwrap()
        } else {
            0
        };

        let flap_sound = audio::load_audio_source(ctx, load_b!("sounds/boom.mp3").unwrap());
        let die_sound = audio::load_audio_source(ctx, load_b!("sounds/die.mp3").unwrap());
        let hit_sound = audio::load_audio_source(ctx, load_b!("sounds/hit.mp3").unwrap());
        let point_sound = audio::load_audio_source(ctx, load_b!("sounds/point.mp3").unwrap());

        let die_timer = time::Timer::new(DIE_TIMER_DURATION);

        Self {
            state: GameState::StartMenu,
            score: 0,

            camera,
            camera_buffer,

            sprite_renderer,

            sprite_atlas,
            ui_renderer,
            gizmo_renderer,

            highscore,
            flap_sound,
            die_sound,
            hit_sound,
            point_sound,
            die_timer,

            // entities
            entities,
            player,
            pipe_middle,
            base_middle,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(feature = "hot_reload")]
        if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::F1) {
            gbase::hot_reload::hot_restart(ctx);
            println!("hot restart");
        }

        let gui_root = Widget::new()
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

                if input::key_just_pressed(ctx, input::KeyCode::Space) {
                    self.score = 0;
                    self.state = GameState::Game;

                    self.entities.get_entity_mut(self.player).velocity.y = PLAYER_JUMP_VELOCITY;
                    audio::play_audio_source(ctx, &self.flap_sound);
                    self.player.get_mut(&mut self.entities).renderable = Renderable::Animation;
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
                // scroll pipes
                let mid = self.entities.get_entity_mut(self.pipe_middle);
                mid.local_pos.x -= dt * SCROLL_SPEED;
                if mid.local_pos.x <= -(BACKGROUND.pixel_size().x / 2.0 + PIPE.pixel_size().x / 2.0)
                {
                    mid.local_pos.x += BACKGROUND.pixel_size().x + PIPE.pixel_size().x;
                    mid.local_pos.y =
                        random::rand(ctx).range_f32(-PIPE_MAX_OFFSET, PIPE_MAX_OFFSET);
                    mid.local_pos.y += PIPE_BASE_OFFSET;

                    // reset score area
                    mid.score_child.get_mut(&mut self.entities).score_area = true;
                }
                // scroll bases
                let mid = &mut self.base_middle.get_mut(&mut self.entities).local_pos;
                mid.x -= dt * SCROLL_SPEED;
                if mid.x <= -(BACKGROUND.pixel_size().x / 2.0) {
                    mid.x += BACKGROUND.pixel_size().x;
                }
                // player movement
                let player = self.player.get_mut(&mut self.entities);
                player.velocity.y -= 9.82 * dt * PLAYER_FALL_SPEED;
                if input::key_just_pressed(ctx, KeyCode::Space) {
                    player.velocity.y = PLAYER_JUMP_VELOCITY;
                    audio::play_audio_source(ctx, &self.flap_sound);
                }
                player.local_pos += player.velocity * dt;
                player.local_pos.y = player.local_pos.y.clamp(
                    -BACKGROUND.pixel_size().y / 2.0 + BASE.pixel_size().y / 2.0,
                    BACKGROUND.pixel_size().y / 2.0,
                );

                // score check
                for eh in self.entities.get_handles_filtered(|a| a.score_area) {
                    if self.entities.check_collision_handle(self.player, eh) {
                        self.score += 1;
                        audio::play_audio_source(ctx, &self.point_sound);
                        eh.get_mut(&mut self.entities).score_area = false;
                    }
                }

                // collisions
                let mut collided = false;

                for eh in self.entities.get_handles() {
                    if !eh.get(&self.entities).obstacle {
                        continue;
                    }
                    if self.entities.check_collision_handle(self.player, eh) {
                        collided = true;
                        if eh.get(&self.entities).is_pipe {
                            self.player.get_mut(&mut self.entities).velocity = vec2(50.0, 50.0);
                        }
                    }
                }

                if collided {
                    self.state = GameState::GameOver;
                    audio::play_audio_source(ctx, &self.hit_sound);
                    self.die_timer.reset();
                    if self.score > self.highscore {
                        self.highscore = self.score;
                        filesystem::store_str(ctx, HIGHSCORE_PATH, &self.score.to_string())
                            .unwrap();
                    }
                    self.player.get_mut(&mut self.entities).renderable = Renderable::Sprite;
                }
            }
            GameState::GameOver => {
                gui_root
                    .main_axis_alignment(Alignment::Start)
                    .gap(10.0)
                    .layout(&mut self.ui_renderer, |renderer| {
                        Widget::new()
                            .height(SizeKind::PercentOfParent(0.3))
                            .render(renderer);
                        Widget::new()
                            .text("Game over")
                            .width(SizeKind::TextSize)
                            .height(SizeKind::TextSize)
                            .text_font_size(100.0)
                            .text_color(gbase_utils::WHITE)
                            .render(renderer);
                        Widget::new()
                            .text(format!("Highscore: {}", self.highscore))
                            .width(SizeKind::TextSize)
                            .height(SizeKind::TextSize)
                            .text_font_size(50.0)
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

                let on_ground = self
                    .entities
                    .check_collision_handle(self.player, self.base_middle);

                let player = self.player.get_mut(&mut self.entities);
                player.velocity.y -= 9.82 * dt * PLAYER_FALL_SPEED;
                player.local_pos += player.velocity * dt;
                player.local_pos.y = player.local_pos.y.clamp(
                    -BACKGROUND.pixel_size().y / 2.0 + BASE.pixel_size().y / 2.0,
                    BACKGROUND.pixel_size().y / 2.0,
                );
                if on_ground {
                    player.velocity.x = 0.0;
                }

                if !on_ground && self.die_timer.just_ticked() {
                    audio::play_audio_source(ctx, &self.die_sound);
                }

                if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
                    self.state = GameState::StartMenu;
                    let player = self.entities.get_entity_mut(self.player);
                    player.velocity = Vec2::ZERO;
                    player.local_pos = vec2(-BIRD_FLAP_0.pixel_size().x, 0.0);

                    let pipe_mid = self.entities.get_entity_mut(self.pipe_middle);
                    pipe_mid.local_pos = vec2(BACKGROUND.w as f32, 0.0);
                }
            }
        }

        let player = self.player.get_mut(&mut self.entities);
        player.local_rotation = match self.state {
            GameState::StartMenu => 0.0,
            GameState::Game | GameState::GameOver => {
                remap(player.velocity.y, -400.0, 100.0, -PI / 2.0, PI / 4.0)
                    .clamp(-PI / 2.0, PI / 4.0)
            }
        };

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        // draw background stencil
        self.sprite_renderer.draw_sprite(
            &Transform2D::from_scale(BACKGROUND.pixel_size()),
            BACKGROUND.atlas_pos(),
            BACKGROUND.atlas_size(),
        );
        self.sprite_renderer
            .render_stencil(ctx, &self.camera_buffer, STENCIL_FULL);

        // background
        self.sprite_renderer.draw_sprite(
            &Transform2D::from_scale(BACKGROUND.pixel_size()),
            BACKGROUND.atlas_pos(),
            BACKGROUND.atlas_size(),
        );

        // draw entities
        for eh in self.entities.get_handles() {
            match eh.get(&self.entities).renderable {
                Renderable::None => {}
                Renderable::Sprite => {
                    let pos = eh.get(&self.entities).pos(&self.entities);
                    let e = eh.get(&self.entities);
                    self.sprite_renderer.draw_sprite(
                        &Transform2D::new(pos, e.local_rotation, e.local_scale),
                        e.sprite.atlas_pos,
                        e.sprite.atlas_size,
                    );
                }
                Renderable::Animation => {
                    let pos = eh.get(&self.entities).pos(&self.entities);
                    let e = eh.get_mut(&mut self.entities);
                    e.animation.tick();
                    self.sprite_renderer.draw_sprite(
                        &Transform2D::new(pos, e.local_rotation, e.local_scale),
                        e.animation.sprites[e.animation.current].atlas_pos,
                        e.animation.sprites[e.animation.current].atlas_size,
                    );
                }
            }
        }
        self.sprite_renderer.render(
            ctx,
            screen_view,
            &self.camera_buffer,
            &self.sprite_atlas,
            STENCIL_FULL,
        );

        // draw debug views
        if input::key_pressed(ctx, KeyCode::KeyG) {
            for e in self.entities.entities.iter() {
                match e.collision {
                    Collision::None => {}
                    Collision::Circle { radius } => {
                        self.gizmo_renderer.draw_circle(
                            &Transform3D::new(
                                e.pos(&self.entities).extend(0.0),
                                Quat::IDENTITY,
                                Vec3::ONE * radius * 2.0,
                            ),
                            if e.score_area {
                                gbase_utils::GREEN.xyz()
                            } else {
                                gbase_utils::RED.xyz()
                            },
                        );
                    }
                    Collision::Quad { size } => {
                        self.gizmo_renderer.draw_quad(
                            &Transform3D::new(
                                e.pos(&self.entities).extend(0.0),
                                Quat::IDENTITY,
                                size.extend(0.0),
                            ),
                            if e.score_area {
                                gbase_utils::GREEN.xyz()
                            } else {
                                gbase_utils::RED.xyz()
                            },
                        );
                    }
                }
            }
        }

        // render to screen
        self.ui_renderer
            .render(ctx, screen_view, render::surface_format(ctx));
        self.gizmo_renderer.render(
            ctx,
            screen_view,
            render::surface_format(ctx),
            &self.camera_buffer,
        );

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut gbase::Context, new_size: gbase::winit::dpi::PhysicalSize<u32>) {
        self.ui_renderer.resize(ctx, new_size);
        self.gizmo_renderer.resize(ctx, new_size);
        self.sprite_renderer.resize(ctx, new_size);
    }
}

impl AtlasSprite {
    pub fn pixel_size(&self) -> Vec2 {
        vec2(self.w as f32, self.h as f32)
    }
    pub fn atlas_pos(&self) -> Vec2 {
        vec2(
            self.x as f32 / sprite_atlas::ATLAS_WIDTH as f32,
            self.y as f32 / sprite_atlas::ATLAS_HEIGHT as f32,
        )
    }
    pub fn atlas_size(&self) -> Vec2 {
        vec2(
            self.w as f32 / sprite_atlas::ATLAS_WIDTH as f32,
            self.h as f32 / sprite_atlas::ATLAS_HEIGHT as f32,
        )
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
