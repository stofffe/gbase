use crate::ui_layout::UIElement;
use core::f32;
use gbase::{
    arc::ArcHandleRuntime,
    asset::{
        AssetCache, AssetHandle, AssetLoader, EmptyError, LoadContext, ShaderGpuLoader,
        ShaderGpuLoaderSettings, ShaderLoader, ShaderLoaderSettings,
    },
    bytemuck, filesystem,
    glam::{self, Mat4},
    input,
    render::{self, ArcHandle, ArcShaderModule, BindGroupBindable, RenderRuntime},
    tracing, wgpu,
};
use std::{collections::HashMap, hash::Hash, path::PathBuf};

/// UI renderer that uses sdf for text
pub struct UIRenderer {
    shader_gpu_handle: AssetHandle<ArcShaderModule>,
    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline_layout: render::ArcPipelineLayout,

    instance_buffer: render::RawBuffer<UIElementInstace>,

    projection: render::UniformBuffer<glam::Mat4>,

    pub font_handle: AssetHandle<Font>,
    pub font_atlas_handle: AssetHandle<SdfFontAtlas>,

    // TODO: should this be some hashable f32 instead?
    font_atlas_supported_chars: Vec<char>,
}

impl UIRenderer {
    pub fn new(
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        font_path: impl Into<PathBuf>,
        font_atlas_raster_size: u32,
        max_elements: u64,
    ) -> Self {
        let mut font_atlas_supported_chars = Vec::new();
        for char in 'a'..='z' {
            font_atlas_supported_chars.push(char);
        }
        for char in 'A'..='Z' {
            font_atlas_supported_chars.push(char);
        }
        for char in '0'..='9' {
            font_atlas_supported_chars.push(char);
        }
        for char in " \t\n,.;/".chars() {
            font_atlas_supported_chars.push(char);
        }

        let font_handle = cache.load_asset::<FontLoader>(&FontLoaderSettings::new(font_path));
        let font_atlas_handle =
            cache.load_asset::<SdfFontAtlasLoader>(&SdfFontAtlasLoaderSettings::new(
                font_handle.clone(),
                font_atlas_supported_chars.clone(),
                font_atlas_raster_size,
            ));

        //
        // gpu resources
        //

        let shader_handle = cache
            .load_asset::<ShaderLoader>(&ShaderLoaderSettings::from_path("assets/shaders/ui.wgsl"));
        let shader_gpu_handle =
            cache.load_asset::<ShaderGpuLoader>(&ShaderGpuLoaderSettings::new(shader_handle));

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // projection
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // atlas texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .vertex()
                    .fragment(),
                // atlas sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let instance_buffer = render::RawBufferBuilder::new(max_elements).build(ctx);

        let projection = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_gpu_handle,
            instance_buffer,
            projection,

            font_handle,
            font_atlas_handle,
            font_atlas_supported_chars,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        ui_elements: &[UIElement],
    ) {
        let Ok(shader) = cache.get_asset(&self.shader_gpu_handle).cloned() else {
            return;
        };

        let Ok(font_atlas) = cache.get_asset(&self.font_atlas_handle) else {
            return;
        };

        if input::key_just_pressed(ctx, input::KeyCode::KeyD) {
            dbg!(&font_atlas.lookup);
        }

        //
        // convert
        //
        let mut instances = Vec::new();
        for elem in 0..ui_elements.len() {
            let element = &ui_elements[elem];

            // container
            instances.push(UIElementInstace {
                element_type: ELEMENT_TYPE_CONTAINER,

                position: [element.x, element.y],
                size: [element.preferred_width, element.preferred_height],
                color: element.background_color.to_array(),
                font_atlas_offset: [0.0, 0.0],
                font_atlas_size: [0.0, 0.0],
            });

            // glyphs
            let atlas_width = font_atlas.texture.width();
            let atlas_height = font_atlas.texture.height();
            if !element.text_info.text.is_empty() {
                for glyph in element.text_layout.glyphs.iter() {
                    // TODO: backup glyph
                    let glyph_info = font_atlas
                        .lookup
                        .get(&glyph.character)
                        .expect("could not find glyph");

                    // All values related to atlas must be scaled
                    // the generation had a large fixed font size, not we just scale it linearly
                    let scale = element.text_info.font_size as f32 / font_atlas.raster_size as f32;
                    let sdf_padding = font_atlas.sdf_params.pad as f32;
                    let x = element.x + glyph.x - sdf_padding * scale;
                    let y = element.y + glyph.y - sdf_padding * scale;
                    let width = glyph_info.atlas_width as f32 * scale;
                    let height = glyph_info.atlas_height as f32 * scale;
                    instances.push(UIElementInstace {
                        element_type: ELEMENT_TYPE_GLYPH,

                        position: [x, y],
                        size: [width, height],
                        color: element.text_info.text_color.to_array(),
                        font_atlas_offset: [
                            glyph_info.atlas_offset_x as f32 / atlas_width as f32,
                            glyph_info.atlas_offset_y as f32 / atlas_height as f32,
                        ],
                        font_atlas_size: [
                            glyph_info.atlas_width as f32 / atlas_width as f32,
                            glyph_info.atlas_height as f32 / atlas_height as f32,
                        ],
                    });
                }
            }
        }

        self.instance_buffer.write(ctx, &instances);
        let screen_size = render::surface_size(ctx);
        self.projection.write(
            ctx,
            &Mat4::orthographic_rh(
                0.0,
                screen_size.width as f32,
                screen_size.height as f32,
                0.0,
                0.0,
                1.0,
            ),
        );

        let atlas_view = render::TextureViewBuilder::new(font_atlas.texture.clone()).build(ctx);
        let atlas_sampler = render::SamplerBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera projection
                self.projection.bindgroup_entry(),
                // atlas texture
                render::BindGroupEntry::Texture(atlas_view),
                // atlas sampler
                render::BindGroupEntry::Sampler(atlas_sampler),
            ])
            .build(ctx);

        let pipeline =
            render::RenderPipelineBuilder::new(shader.clone(), self.pipeline_layout.clone())
                .buffers(vec![UIElementInstace::desc()])
                .topology(wgpu::PrimitiveTopology::TriangleStrip)
                .single_target(
                    render::ColorTargetState::new()
                        .format(view_format)
                        .blend(wgpu::BlendState::ALPHA_BLENDING),
                )
                .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                pass.draw(0..4, 0..instances.len() as u32);
            });
    }
}

#[derive(Debug, Clone)]
pub struct AtlasGlyphInfo {
    pub atlas_offset_x: usize,
    pub atlas_offset_y: usize,
    pub atlas_width: usize,
    pub atlas_height: usize,
}

fn create_font_atlas(
    render_runtime: impl AsRef<RenderRuntime>,
    arc_runtime: impl AsRef<ArcHandleRuntime>,
    font: &fontdue::Font,
    supported_chars: &[char],
    font_raster_size: f32,
    sdf_params: sdfer::esdt::Params,
) -> (HashMap<char, AtlasGlyphInfo>, render::ArcTexture) {
    let mut glyphs = Vec::new();
    let mut total_area = 0;

    // create sdf glyphs
    for &char in supported_chars {
        let (font_metrics, font_raster) = font.rasterize(char, font_raster_size);
        let is_invisble_char = font_metrics.width == 0 || font_metrics.height == 0;
        if is_invisble_char {
            glyphs.push((char, font_metrics.width, font_metrics.height, None));
        } else {
            // u8 array -> sdfer
            let mut sdf_input =
                sdfer::Image2d::<sdfer::Unorm8>::new(font_metrics.width, font_metrics.height);
            for y in 0..font_metrics.height {
                for x in 0..font_metrics.width {
                    let v = font_raster[y * font_metrics.width + x];
                    sdf_input[(x, y)] = sdfer::Unorm8::from_bits(v);
                }
            }

            // generate glyph sdf
            //
            let (sdf_output, _) = sdfer::esdt::glyph_to_sdf(&mut sdf_input, sdf_params, None);
            let sdf_width = sdf_output.width();
            let sdf_height = sdf_output.height();

            // sdfer -> u8 array
            let mut sdf_raster = vec![0u8; sdf_width * sdf_height];
            for y in 0..sdf_height {
                for x in 0..sdf_width {
                    sdf_raster[y * sdf_width + x] = sdf_output[(x, y)].to_bits();
                }
            }

            total_area += sdf_width * sdf_height;
            glyphs.push((char, sdf_width, sdf_height, Some(sdf_raster)));
        }
    }

    // sort by glyph height
    glyphs.sort_by_key(|(_, _, height, _)| *height);

    // packing
    // split up packing and atlas creation since packing might resize

    // calculate atlas size
    let mut atlas_glyph_lookup = HashMap::new();
    let mut atlas_side_size = ((total_area as f32).sqrt() as usize).next_power_of_two(); // TODO: cleanup
    let mut atlas_offset_x;
    let mut atlas_offset_y;
    let mut row_y_max;

    loop {
        let mut packing_success = true;
        atlas_offset_x = 0;
        atlas_offset_y = 0;
        row_y_max = 0;

        for (char, width, height, _) in glyphs.iter() {
            // check for wrapping
            if width + atlas_offset_x > atlas_side_size {
                // check for space left
                if height + atlas_offset_y > atlas_side_size {
                    packing_success = false;
                    break;
                }

                atlas_offset_y += row_y_max;
                row_y_max = 0;
                atlas_offset_x = 0;
            }

            atlas_glyph_lookup.insert(
                *char,
                AtlasGlyphInfo {
                    atlas_offset_x,
                    atlas_offset_y,

                    atlas_width: *width,
                    atlas_height: *height,
                },
            );

            atlas_offset_x += width;
            row_y_max = row_y_max.max(*height);
        }

        if packing_success {
            break;
        } else {
            atlas_side_size *= 2;
            atlas_glyph_lookup.clear();
        }
    }

    // atlas creation using packing info
    let atlas_size = atlas_side_size * atlas_side_size;
    let mut atlas_data = vec![0u8; atlas_size];
    for &(char, width, height, ref raster) in glyphs.iter() {
        // skip invisible chars such as space, tab...
        let Some(raster) = raster else {
            continue;
        };

        let glyph_info = atlas_glyph_lookup.get(&char).expect("could not find glyph");

        for glyph_x in 0..width {
            for glyph_y in 0..height {
                let index = glyph_x + glyph_y * width;
                let value = raster[index];

                let atlas_x = glyph_info.atlas_offset_x + glyph_x;
                let atlas_y = glyph_info.atlas_offset_y + glyph_y;
                let index = atlas_x + atlas_y * atlas_side_size;
                atlas_data[index] = value;
            }
        }
    }

    let font_atlas = render::TextureBuilder::new()
        .with_format(wgpu::TextureFormat::R8Unorm)
        .build(
            &render_runtime.as_ref().device,
            &render_runtime.as_ref().queue,
            render::TextureSource::Data(atlas_side_size as u32, atlas_side_size as u32, atlas_data),
        );
    let arc_font_atlas = ArcHandle::new(arc_runtime, font_atlas);

    (atlas_glyph_lookup, arc_font_atlas)
}

const ELEMENT_TYPE_CONTAINER: u32 = 0;
const ELEMENT_TYPE_GLYPH: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UIElementInstace {
    // flag containing type of element
    // container: 0
    // glyph: 1
    pub element_type: u32,

    // uv coordinate system, (0,0) top left and y+ is down
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],

    // fonts
    pub font_atlas_offset: [f32; 2],
    pub font_atlas_size: [f32; 2],
}

impl UIElementInstace {
    pub fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout::from_vertex_formats(
            wgpu::VertexStepMode::Instance,
            vec![
                wgpu::VertexFormat::Uint32,    // element type
                wgpu::VertexFormat::Float32x2, // pos
                wgpu::VertexFormat::Float32x2, // scale
                wgpu::VertexFormat::Float32x4, // color
                wgpu::VertexFormat::Float32x2, // atlas offset
                wgpu::VertexFormat::Float32x2, // atlas size
            ],
        )
    }
}

// TODO: keep or nah?
#[derive(Clone)]
pub struct Font {
    pub font: fontdue::Font,
}

pub struct FontLoader {}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct FontLoaderSettings {
    path: PathBuf,

    pub font_collection_index: u32,
    pub font_scale: u32,
    pub font_load_substitutions: bool,
}

impl FontLoaderSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            font_collection_index: 0,
            font_scale: 40,
            font_load_substitutions: true,
        }
    }
}

impl AssetLoader for FontLoader {
    type Asset = Font;
    type Settings = FontLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = load_ctx.load_bytes(&settings.path).await?;
        let font = fontdue::Font::from_bytes(
            bytes,
            fontdue::FontSettings {
                collection_index: settings.font_collection_index,
                scale: settings.font_scale as f32,
                load_substitutions: settings.font_load_substitutions,
            },
        )
        .expect("could not create font from bytes");

        Ok(Font { font })
    }
}

// TODO: no cpu representation currently, directly to gpu instead
#[derive(Clone)]
pub struct SdfFontAtlas {
    pub lookup: HashMap<char, AtlasGlyphInfo>,
    pub texture: render::ArcTexture,
    pub raster_size: u32,
    pub sdf_params: sdfer::esdt::Params,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct SdfFontAtlasLoaderSettings {
    font_handle: AssetHandle<Font>,
    supported_chars: Vec<char>,
    font_raster_size: u32,
}
impl SdfFontAtlasLoaderSettings {
    fn new(
        font_handle: AssetHandle<Font>,
        supported_chars: Vec<char>,
        font_raster_size: u32,
    ) -> Self {
        Self {
            font_handle,
            supported_chars,
            font_raster_size,
        }
    }
}

pub struct SdfFontAtlasLoader;

impl AssetLoader for SdfFontAtlasLoader {
    type Asset = SdfFontAtlas;
    type Settings = SdfFontAtlasLoaderSettings;
    type Error = EmptyError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let raster_size = settings.font_raster_size;
        let font = load_ctx.request_get(&settings.font_handle).await;

        // TODO: hardcoded
        let sdf_params = sdfer::esdt::Params {
            pad: 8,
            radius: 8.0,
            cutoff: 0.5,
            solidify: true,
            preprocess: false,
        };

        let (lookup, texture) = create_font_atlas(
            load_ctx.render_runtime(),
            load_ctx.arc_runtime(),
            &font.font,
            &settings.supported_chars,
            raster_size as f32,
            sdf_params,
        );

        Ok(SdfFontAtlas {
            lookup,
            texture,
            raster_size,
            sdf_params,
        })
    }
}
