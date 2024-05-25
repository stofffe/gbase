use core::panic;
use encase::ShaderType;
use gbase::{
    filesystem, input,
    render::{self, Transform, VertexFull, VertexUV},
    Callbacks, Context,
};
use glam::{vec3, Quat, Vec3};

#[pollster::main]
async fn main() {
    let (ctx, ev) = gbase::ContextBuilder::new().build().await;
    let app = App::new(&ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    model1: Model,
    material1: Material,
    model1_transform: Transform,
    model1_transform_uni: render::UniformBuffer,
    // mesh2: Mesh,
    pipeline: wgpu::RenderPipeline,
    bindgroup: wgpu::BindGroup,
    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer,

    light: Vec3,
    light_buffer: render::UniformBuffer,

    depth_buffer: render::DepthBuffer,

    gizmo_renderer: render::GizmoRenderer,

    debug_input: render::DebugInput,

    // Framebuffers
    deferred_buffers: DeferredBuffers,
    deferred_renderer: DeferredRenderer,
    // albedo_buffer: render::ResizableFrameBuffer,
    // normal_buffer: render::ResizableFrameBuffer,
    // roughness_buffer: render::ResizableFrameBuffer,
    texture_renderer: render::TextureRenderer,
}

impl App {
    async fn new(ctx: &Context) -> Self {
        let (model1, material1) =
            glb_to_vertex_mesh(ctx, include_bytes!("../assets/ak47.glb")).await;

        let albedo_texture = material1.albedo.as_ref().unwrap();
        let normal_texture = material1.normal.as_ref().unwrap();
        let roughness_texture = material1.roughness.as_ref().unwrap();

        let model1_transform = render::Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(2.0));
        let model1_transform_uni =
            render::UniformBufferBuilder::new().build(ctx, render::TransformUniform::min_size());

        let sampler = render::SamplerBuilder::new().build(ctx);
        let shader_str = filesystem::load_string(ctx, "mesh.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new(&shader_str).build(ctx);
        let camera_buffer = render::UniformBufferBuilder::new()
            .build(ctx, render::PerspectiveCameraUniform::min_size());
        let light = Vec3::ZERO;
        let light_buffer = render::UniformBufferBuilder::new().build_init(ctx, &light);

        let debug_input = render::DebugInput::new(ctx);
        let (bindgroup_layoyt, bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                // camera
                render::BindGroupCombinedEntry::new(camera_buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
                // normal
                render::BindGroupCombinedEntry::new(normal_texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(normal_texture.binding_type()),
                // albedo
                render::BindGroupCombinedEntry::new(albedo_texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(albedo_texture.binding_type()),
                // roughness
                render::BindGroupCombinedEntry::new(roughness_texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(roughness_texture.binding_type()),
                // sampler
                render::BindGroupCombinedEntry::new(sampler.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(sampler.binding_filtering()),
                // light
                render::BindGroupCombinedEntry::new(light_buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
                // transform
                render::BindGroupCombinedEntry::new(model1_transform_uni.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX)
                    .uniform(),
                // debug input
                render::BindGroupCombinedEntry::new(debug_input.buffer().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
            ])
            .build(ctx);
        let position_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let albedo_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let normal_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let roughness_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let deferred_buffers = DeferredBuffers {
            position: position_buffer,
            albedo: albedo_buffer,
            normal: normal_buffer,
            roughness: roughness_buffer,
        };
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .buffers(&[model1.meshes[0].vertex_buffer.desc()])
            .targets(&[
                Some(deferred_buffers.position.target()),
                Some(deferred_buffers.albedo.target()),
                Some(deferred_buffers.normal.target()),
                Some(deferred_buffers.roughness.target()),
            ])
            .bind_groups(&[&bindgroup_layoyt])
            .depth_stencil(render::DepthBuffer::depth_stencil_state())
            .cull_mode(wgpu::Face::Back)
            .build(ctx);
        let camera = render::PerspectiveCamera::new();
        let depth_buffer = render::DepthBuffer::new(ctx);
        let gizmo_renderer = render::GizmoRenderer::new(ctx);

        let texture_renderer = render::TextureRenderer::new(ctx).await;
        let deferred_renderer =
            DeferredRenderer::new(ctx, &deferred_buffers, &camera_buffer, &light_buffer).await;
        Self {
            model1,
            material1,
            model1_transform,
            model1_transform_uni,
            // mesh2,
            pipeline,
            bindgroup,
            camera,
            camera_buffer,
            depth_buffer,
            light,
            light_buffer,
            gizmo_renderer,
            debug_input,

            texture_renderer,
            deferred_buffers,
            deferred_renderer,
        }
    }
}

impl Callbacks for App {
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        // Update light pos
        let t = gbase::time::time_since_start(ctx);
        // self.light = vec3(t.sin() * 5.0, 0.0, t.cos() * 5.0);
        self.light = vec3(5.0, 1.5, 5.0);
        self.light_buffer.write(ctx, &self.light);
        // Update camera
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        // Update transform
        self.model1_transform = Transform::new(Vec3::ZERO, Quat::from_rotation_y(t), Vec3::ONE);
        self.model1_transform_uni
            .write(ctx, &self.model1_transform.uniform());
        // Update debug input
        self.debug_input.update_buffer(ctx);

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        // Render albedo
        let mut mesh_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: self.deferred_buffers.position.view(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: self.deferred_buffers.albedo.view(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: self.deferred_buffers.normal.view(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: self.deferred_buffers.roughness.view(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                }),
            ],
            depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_clear()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        mesh_pass.set_pipeline(&self.pipeline);
        mesh_pass.set_bind_group(0, &self.bindgroup, &[]);

        for mesh in self.model1.meshes.iter() {
            mesh_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            mesh_pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_buffer.format());
            mesh_pass.draw_indexed(0..mesh.index_buffer.len(), 0, 0..1);
        }
        drop(mesh_pass);

        // let mut tex = self.deferred_buffers.albedo.texture();
        // // let mut tex = self.material1.other[0].texture();
        // if input::key_pressed(ctx, input::KeyCode::F1) {
        //     tex = self.deferred_buffers.normal.texture();
        // } else if input::key_pressed(ctx, input::KeyCode::F2) {
        //     tex = self.deferred_buffers.roughness.texture();
        // } else if input::key_pressed(ctx, input::KeyCode::F3) {
        //     tex = self.material1.albedo.as_ref().unwrap().texture();
        // } else if input::key_pressed(ctx, input::KeyCode::F4) {
        //     tex = self.material1.normal.as_ref().unwrap().texture();
        // } else if input::key_pressed(ctx, input::KeyCode::F5) {
        //     tex = self.material1.roughness.as_ref().unwrap().texture();
        // }
        // self.texture_renderer
        //     .render(ctx, screen_view, &mut encoder, tex);
        self.deferred_renderer
            .render(ctx, screen_view, &mut encoder);

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));

        self.gizmo_renderer.draw_sphere(
            0.1,
            &render::Transform::new(self.light, Quat::IDENTITY, Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );
        self.gizmo_renderer
            .render(ctx, screen_view, &mut self.camera);

        false

        // Render
        // let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //     label: None,
        //     color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //         view: screen_view,
        //         ops: wgpu::Operations {
        //             load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        //             store: wgpu::StoreOp::Store,
        //         },
        //         resolve_target: None,
        //     })],
        //     depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_clear()),
        //     timestamp_writes: None,
        //     occlusion_query_set: None,
        // });
        //
        // render_pass.set_pipeline(&self.pipeline);
        // render_pass.set_bind_group(0, &self.bindgroup, &[]);
        //
        // for mesh in self.model1.meshes.iter() {
        //     render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        //     render_pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_buffer.format());
        //     render_pass.draw_indexed(0..mesh.index_buffer.len(), 0, 0..1);
        // }
        //
        // drop(render_pass);
        //
        // let queue = render::queue(ctx);
        // queue.submit(Some(encoder.finish()));
        //
        // // Gizmos
        // self.gizmo_renderer.draw_sphere(
        //     0.1,
        //     &render::Transform::new(self.light, Quat::IDENTITY, Vec3::ONE),
        //     vec3(1.0, 0.0, 0.0),
        // );
        // self.gizmo_renderer
        //     .render(ctx, screen_view, &mut self.camera);
        //
        // false
    }

    fn resize(&mut self, ctx: &mut Context) {
        self.depth_buffer.resize(ctx);
        self.gizmo_renderer.resize(ctx);
        self.texture_renderer.resize(ctx);

        self.deferred_buffers.resize(ctx);
        self.deferred_renderer.resize(
            ctx,
            &self.deferred_buffers,
            &self.camera_buffer,
            &self.light_buffer,
        );
    }
    fn init(&mut self, _ctx: &mut Context) {
        self.camera.pos = vec3(0.5, 0.0, 1.0);
    }
    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = gbase::time::delta_time(ctx);

        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            self.camera.yaw = 0.0;
            self.camera.pitch = 0.0;
        }

        // Camera rotation
        if input::mouse_button_pressed(ctx, input::MouseButton::Left) {
            let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
            self.camera.yaw -= 1.0 * dt * mouse_dx;
            self.camera.pitch -= 1.0 * dt * mouse_dy;
        }

        // Camera movement
        let mut camera_movement_dir = Vec3::ZERO;
        if input::key_pressed(ctx, input::KeyCode::KeyW) {
            camera_movement_dir += self.camera.forward();
        }

        if input::key_pressed(ctx, input::KeyCode::KeyS) {
            camera_movement_dir -= self.camera.forward();
        }
        if input::key_pressed(ctx, input::KeyCode::KeyA) {
            camera_movement_dir -= self.camera.right();
        }
        if input::key_pressed(ctx, input::KeyCode::KeyD) {
            camera_movement_dir += self.camera.right();
        }
        if camera_movement_dir != Vec3::ZERO {
            self.camera.pos += camera_movement_dir.normalize() * dt;
        }

        // Camera zoom
        let (_, scroll_y) = input::scroll_delta(ctx);
        self.camera.fov += scroll_y * dt;

        false
    }
}

//
// Deferred renderer
//

struct DeferredBuffers {
    position: render::ResizableFrameBuffer,
    albedo: render::ResizableFrameBuffer,
    normal: render::ResizableFrameBuffer,
    roughness: render::ResizableFrameBuffer,
}

impl DeferredBuffers {
    fn resize(&mut self, ctx: &Context) {
        self.position.resize(ctx);
        self.albedo.resize(ctx);
        self.normal.resize(ctx);
        self.roughness.resize(ctx);
    }
}

struct DeferredRenderer {
    pipeline: wgpu::RenderPipeline,
    bindgroup: wgpu::BindGroup,

    vertex_buffer: render::VertexBuffer<VertexUV>,
    debug_input: render::DebugInput,
}

impl DeferredRenderer {
    async fn new(
        ctx: &Context,
        buffers: &DeferredBuffers,
        camera: &render::UniformBuffer,
        light: &render::UniformBuffer,
    ) -> Self {
        let shader_str = filesystem::load_string(ctx, "deferred.wgsl").await.unwrap();
        let vertex_buffer = render::VertexBufferBuilder::new(QUAD_VERTICES).build(ctx);
        let shader = render::ShaderBuilder::new(&shader_str).build(ctx);
        let debug_input = render::DebugInput::new(ctx);
        let (bindgroup_layout, bindgroup) =
            Self::bindgroups(ctx, buffers, camera, light, &debug_input);
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .bind_groups(&[&bindgroup_layout])
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .buffers(&[vertex_buffer.desc()])
            .build(ctx);
        Self {
            pipeline,
            bindgroup,
            vertex_buffer,
            debug_input,
        }
    }

    fn render(
        &mut self,
        ctx: &Context,
        screen_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        self.debug_input.update_buffer(ctx);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bindgroup, &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
    }
    fn bindgroups(
        ctx: &Context,
        buffers: &DeferredBuffers,
        camera: &render::UniformBuffer,
        light: &render::UniformBuffer,
        debug_input: &render::DebugInput,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let sampler = render::SamplerBuilder::new().build(ctx);
        render::BindGroupCombinedBuilder::new()
            .label("deferred")
            .entries(&[
                //sampler
                render::BindGroupCombinedEntry::new(sampler.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(sampler.binding_nonfiltering()),
                // position
                render::BindGroupCombinedEntry::new(buffers.position.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.position.binding_type_nonfilter()),
                // albedo
                render::BindGroupCombinedEntry::new(buffers.albedo.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.albedo.binding_type_nonfilter()),
                // normal
                render::BindGroupCombinedEntry::new(buffers.normal.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.normal.binding_type_nonfilter()),
                // roughness
                render::BindGroupCombinedEntry::new(buffers.roughness.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.roughness.binding_type_nonfilter()),
                // camera
                render::BindGroupCombinedEntry::new(camera.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
                // light
                render::BindGroupCombinedEntry::new(light.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
                // debug input
                render::BindGroupCombinedEntry::new(debug_input.buffer().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
            ])
            .build(ctx)
    }
    fn resize(
        &mut self,
        ctx: &Context,
        buffers: &DeferredBuffers,
        camera: &render::UniformBuffer,
        light: &render::UniformBuffer,
    ) {
        let (_, bindgroup) = Self::bindgroups(ctx, buffers, camera, light, &self.debug_input);
        self.bindgroup = bindgroup;
    }
}

//
// Mesh
//

struct Model {
    meshes: Vec<Mesh>,
}

#[derive(Default)]
struct Material {
    albedo: Option<render::Texture>,
    normal: Option<render::Texture>,
    roughness: Option<render::Texture>,
    other: Vec<render::Texture>,
}

struct Mesh {
    vertex_buffer: render::VertexBuffer<render::VertexFull>,
    index_buffer: render::IndexBuffer,
}

async fn glb_to_vertex_mesh(ctx: &Context, glb_bytes: &[u8]) -> (Model, Material) {
    let glb = gltf::Glb::from_slice(glb_bytes).unwrap();
    let info = gltf::Gltf::from_slice(&glb.json).unwrap();
    let buffer = glb.bin.expect("no buffer");

    let mut meshes = Vec::new();
    let mut material = Material::default();
    for mesh in info.meshes() {
        for prim in mesh.primitives() {
            // Load indices
            let view = prim.indices().unwrap().view().unwrap();
            let (ind_size, ind_off) = (view.length(), view.offset());
            let indices = match (
                prim.indices().unwrap().data_type(),
                prim.indices().unwrap().dimensions(),
            ) {
                (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Scalar) => {
                    let inds: &[u8] = bytemuck::cast_slice(&buffer[ind_off..ind_off + ind_size]);
                    inds.iter().map(|&i| i as u32).collect::<Vec<_>>()
                }
                (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Scalar) => {
                    let inds: &[u16] = bytemuck::cast_slice(&buffer[ind_off..ind_off + ind_size]);
                    inds.iter().map(|&i| i as u32).collect::<Vec<_>>()
                }
                (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Scalar) => {
                    let inds: &[u32] = bytemuck::cast_slice(&buffer[ind_off..ind_off + ind_size]);
                    inds.to_vec()
                }
                form => {
                    panic!("cringe index format {form:?}")
                }
            };

            // Load pos, albedo, normal, tangent
            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut tangents = Vec::new();
            let mut uvs = Vec::new();

            for (sem, acc) in prim.attributes() {
                let view = acc.view().unwrap();
                let offset = acc.offset() + view.offset();
                let size = acc.count() * acc.size();
                let typ = acc.data_type();
                let dimension = acc.dimensions();

                match (sem, typ, dimension) {
                    (
                        gltf::Semantic::Positions,
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec3,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for pos in buf.chunks(3) {
                            positions.push((pos[0], pos[1], pos[2]));
                        }
                        eprintln!("POS {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::Normals,
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec3,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for normal in buf.chunks(3) {
                            normals.push((normal[0], normal[1], normal[2]))
                        }
                        eprintln!("NORMAL {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::Tangents,
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec4,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for tangent in buf.chunks(4) {
                            tangents.push((tangent[0], tangent[1], tangent[2], tangent[3]));
                            // TODO eprintln!("HAND {}", tangent[3]);
                        }
                        eprintln!("TANGENT {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::Colors(_),
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec3,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        eprintln!("COLOR {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::TexCoords(i),
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec2,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for uv in buf.chunks(2) {
                            uvs.push((uv[0], uv[1]))
                        }
                        eprintln!("UV({i}) {:?}", buf.len());
                    }
                    info => log::warn!("cringe type: {:?}", info),
                }
            }

            let mut vertices = Vec::new();
            for (((pos, normal), uv), tangent) in
                positions.into_iter().zip(normals).zip(uvs).zip(tangents)
            {
                vertices.push(VertexFull {
                    position: [pos.0, pos.1, pos.2],
                    normal: [normal.0, normal.1, normal.2],
                    color: [1.0, 1.0, 1.0],
                    uv: [uv.0, uv.1],
                    tangent: [tangent.0, tangent.1, tangent.2, tangent.3],
                });
            }

            meshes.push(Mesh {
                vertex_buffer: render::VertexBufferBuilder::new(&vertices).build(ctx),
                index_buffer: render::IndexBufferBuilder::new(&indices).build(ctx),
            });

            // Normal texture
            if let Some(normal_texture) = prim.material().normal_texture() {
                if let gltf::image::Source::View { view, .. } =
                    normal_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.normal = Some(
                        render::TextureBuilder::new()
                            .usage(
                                wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_SRC
                                    | wgpu::TextureUsages::COPY_DST,
                            )
                            .build_init(ctx, img_buf),
                    );
                }
            }

            // Albedo texture
            if let Some(base_color_texture) = prim
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                if let gltf::image::Source::View { view, .. } =
                    base_color_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.albedo = Some(
                        render::TextureBuilder::new()
                            .usage(
                                wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_SRC
                                    | wgpu::TextureUsages::COPY_DST,
                            )
                            .build_init(ctx, img_buf),
                    );
                }
            }

            // Metal
            if let Some(roughness_texture) = prim
                .material()
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
            {
                if let gltf::image::Source::View { view, .. } =
                    roughness_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.roughness = Some(
                        render::TextureBuilder::new()
                            .usage(
                                wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_SRC
                                    | wgpu::TextureUsages::COPY_DST,
                            )
                            .build_init(ctx, img_buf),
                    );
                }
            }
        }

        // eprintln!("IMAGES {}", info.images().len());
        // for image in info.images() {
        //     // eprintln!("{:?}", image.source());
        //     match image.source() {
        //         gltf::image::Source::View { view, .. } => {
        //             // let img_buf = &buffer[view.offset()..view.offset() + view.length()];
        //             // material.albedo = Some(render::TextureBuilder::new().build_init(ctx, img_buf));
        //             let img_buf = &buffer[view.offset()..view.offset() + view.length()];
        //             material.other.push(
        //                 render::TextureBuilder::new()
        //                     .usage(wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC)
        //                     .build_init(ctx, img_buf),
        //             );
        //         }
        //         gltf::image::Source::Uri { uri, .. } => {
        //             eprintln!("URI")
        //             // let mut path = PathBuf::from("kenney_survival-kit/Models");
        //             // path.push(uri);
        //             //
        //             // eprintln!("{}", path.to_str().unwrap());
        //             // let img_buf = filesystem::load_bytes(ctx, path).await.unwrap();
        //             //
        //             // // let img_buf = fs::read(uri).unwrap();
        //             // material.albedo = Some(render::TextureBuilder::new().build_init(ctx, &img_buf));
        //         }
        //     }
        // }

        // material
    }

    (Model { meshes }, material)
}

// fn obj_bytes_to_vertex_mesh(ctx: &Context, obj: &[u8], mtl: Option<&[u8]>) -> Model {
//     let (model, mat) = tobj::load_obj_buf(
//         &mut Cursor::new(obj),
//         &tobj::LoadOptions {
//             single_index: true,
//             triangulate: false,
//             ignore_points: false,
//             ignore_lines: false,
//         },
//         |_| {
//             if let Some(mtl) = mtl {
//                 Ok(tobj::load_mtl_buf(&mut Cursor::new(mtl)).expect("could not load mtl"))
//             } else {
//                 Ok(tobj::load_mtl_buf(&mut Cursor::new(&[])).unwrap())
//             }
//         },
//     )
//     .expect("could not load obj");
//     let (models, materials) = (model, mat.expect("could not load mat"));
//
//     // eprintln!("positions: {:?}", model.mesh.positions);
//     // eprintln!("position indicies: {:?}", model.mesh.indices);
//     // eprintln!("normals: {:?}", model.mesh.normals);
//     // eprintln!("normal indices: {:?}", model.mesh.normal_indices);
//     // eprintln!("texcoords: {:?}", model.mesh.texcoords);
//     // eprintln!("texcoord indices: {:?}", model.mesh.texcoord_indices);
//     eprintln!("meshes {}", models.len());
//     for model in &models {
//         // eprintln!("{model:?}");
//         eprintln!("{:?}: {:?}", model.name, model.mesh.material_id);
//     }
//     eprintln!("materials {}", materials.len());
//     for mat in &materials {
//         eprintln!("{mat:?}");
//     }
//
//     let mut meshes = Vec::with_capacity(models.len());
//     for model in models {
//         let mut positions = model.mesh.positions.iter().cloned();
//         let mut colors = model.mesh.vertex_color.iter().cloned();
//         let mut normals = model.mesh.normals.iter().cloned();
//         let mut uvs = model.mesh.texcoords.iter().cloned();
//
//         let mut vertices = Vec::new();
//
//         for _ in (0..positions.len()).step_by(3) {
//             vertices.push(render::VertexFull {
//                 position: [
//                     positions.next().unwrap(),
//                     positions.next().unwrap(),
//                     positions.next().unwrap(),
//                 ],
//                 normal: [
//                     normals.next().unwrap(),
//                     normals.next().unwrap(),
//                     normals.next().unwrap(),
//                 ],
//                 color: [
//                     colors.next().unwrap_or_default(),
//                     colors.next().unwrap_or_default(),
//                     colors.next().unwrap_or_default(),
//                 ],
//                 uv: [
//                     uvs.next().unwrap_or_default(),
//                     1.0 - uvs.next().unwrap_or_default(),
//                 ], // flip uvs
//             });
//         }
//
//         let indices = &model.mesh.indices;
//
//         let vertex_buffer = render::VertexBufferBuilder::new(&vertices).build(ctx);
//         let index_buffer = render::IndexBufferBuilder::new(indices).build(ctx);
//
//         let mesh = Mesh {
//             vertex_buffer,
//             index_buffer,
//         };
//
//         meshes.push(mesh);
//     }
//
//     let mut material = Vec::with_capacity(materials.len());
//     // for mat in materials {
//     //     material.push(Material {});
//     // }
//
//     Model { meshes, material }
// }
#[rustfmt::skip]
const QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
];
