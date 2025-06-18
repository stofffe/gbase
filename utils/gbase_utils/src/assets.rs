use gbase::{
    asset,
    render::{self, ArcHandle, GpuImage, Image},
    tracing,
    wgpu::{self},
    Context,
};
use std::{
    collections::HashMap, fs, marker::PhantomData, sync::atomic::AtomicU64, time::SystemTime,
};

//
// Asset handle
//

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

// TODO: should have type aswell
#[derive(Debug)]
pub struct AssetHandle<T: 'static> {
    id: u64,
    ty: PhantomData<T>,
}

impl<T: 'static> AssetHandle<T> {
    #![allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            ty: PhantomData,
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl<T: 'static> PartialOrd for AssetHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<T: 'static> Ord for AssetHandle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: 'static> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Eq for AssetHandle<T> {}

impl<T: 'static> std::hash::Hash for AssetHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: 'static> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            ty: PhantomData,
        }
    }
}

#[derive(Debug)]
struct ReloadHandle<T: 'static> {
    path: String,
    modified: SystemTime,
    handle: AssetHandle<T>,
}

//
// Generic asset manager
//

pub trait Asset<T: 'static, G: 'static> {
    fn convert(&self, ctx: &mut Context) -> ArcHandle<G>;
    fn reload(&mut self, ctx: &mut Context, data: Vec<u8>);
}

#[derive(Debug)]
pub struct AssetCache<T: 'static, G: 'static> {
    cpu_cache: HashMap<AssetHandle<T>, (T, bool)>,
    gpu_cache: HashMap<AssetHandle<T>, ArcHandle<G>>,
    reload: Vec<ReloadHandle<T>>,
}

impl<T, G> AssetCache<T, G>
where
    G: 'static,
    T: 'static + Asset<T, G>,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            cpu_cache: HashMap::new(),
            gpu_cache: HashMap::new(),
            reload: Vec::new(),
        }
    }

    pub fn allocate(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::new();

        self.cpu_cache.insert(handle.clone(), (data, true));

        handle
    }

    pub fn allocate_reload(&mut self, data: T, reload_path: String) -> AssetHandle<T> {
        let handle = self.allocate(data);

        self.watch(reload_path, handle.clone());

        handle
    }

    pub fn get(&self, id: AssetHandle<T>) -> &T {
        &self.cpu_cache.get(&id).unwrap().0 // fine to unwrap here it think
    }

    pub fn get_mut(&mut self, id: AssetHandle<T>) -> &mut T {
        let (typ, changed) = self.cpu_cache.get_mut(&id).unwrap(); // fine to unwrap here it think
        *changed = true;
        typ
    }

    #[allow(clippy::result_unit_err)]
    pub fn get_gpu(&mut self, ctx: &mut Context, id: AssetHandle<T>) -> ArcHandle<G> {
        debug_assert!(self.cpu_cache.contains_key(&id), "handle doesnt exist");

        match self.gpu_cache.get_mut(&id) {
            // create buffer
            None => {
                let (cpu_typ, changed) = self.cpu_cache.get(&id).expect("handle doesnt exist");
                debug_assert!(*changed);

                let gpu_typ = cpu_typ.convert(ctx);
                self.gpu_cache.insert(id.clone(), gpu_typ);
                tracing::info!("create cached gpu buffer");
            }
            // get cached or update buffer
            Some(gpu_typ) => {
                let (cpu_typ, changed) = self.cpu_cache.get_mut(&id).expect("handle doesnt exist");
                if *changed {
                    *changed = false;
                    *gpu_typ = cpu_typ.convert(ctx);
                    tracing::info!("update cached gpu buffer");
                }
            }
        }

        self.gpu_cache.get_mut(&id).expect("should exist").clone()
    }

    pub fn watch(&mut self, path: String, handle: AssetHandle<T>) {
        let modified = match fs::metadata(&path) {
            Ok(metadata) => metadata.modified().expect("could not get metadata"),
            Err(err) => {
                tracing::warn!("could not get metadata for {}: {}", path, err);
                SystemTime::now()
            }
        };
        self.reload.push(ReloadHandle {
            path,
            modified,
            handle,
        });
    }

    pub fn check_watched_files(&mut self, ctx: &mut Context) {
        for i in 0..self.reload.len() {
            let Ok(md) = fs::metadata(&self.reload[i].path) else {
                tracing::warn!("could not get metadata for {}", &self.reload[i].path);
                continue;
            };

            let modified = md.modified().expect("could not get modified");
            if modified != self.reload[i].modified {
                self.reload[i].modified = modified;

                let bytes = fs::read(&self.reload[i].path).unwrap();
                self.get_mut(self.reload[i].handle.clone())
                    .reload(ctx, bytes);

                tracing::info!("reload {}", self.reload[i].path);
            }
        }
    }
}

// trait which takes hot reloadable (bytes -> update struct)

impl Asset<render::Mesh, render::GpuMesh> for render::Mesh {
    fn convert(&self, ctx: &mut Context) -> ArcHandle<render::GpuMesh> {
        let gpu_mesh = render::GpuMesh::new(ctx, self);
        ArcHandle::new(gpu_mesh)
    }

    fn reload(&mut self, _ctx: &mut Context, _data: Vec<u8>) {
        tracing::warn!("meshes can not currently be hot reloaded");
    }
}

impl Asset<Image, render::GpuImage> for Image {
    fn convert(&self, ctx: &mut Context) -> ArcHandle<GpuImage> {
        let texture = self.texture.clone().build(ctx);
        let sampler = self.sampler.clone().build(ctx);
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
        let image_gpu = render::GpuImage::new(texture, view, sampler);

        ArcHandle::new(image_gpu)
    }

    fn reload(&mut self, _ctx: &mut Context, data: Vec<u8>) {
        let img = image::load_from_memory(&data);

        let Ok(img) = img else {
            tracing::error!("could not decode image bytes");
            return;
        };

        let img = img.to_rgba8();
        self.texture.source = render::TextureSource::Data(img.width(), img.height(), img.to_vec());
    }
}

impl Asset<render::ShaderBuilder, wgpu::ShaderModule> for render::ShaderBuilder {
    fn convert(&self, ctx: &mut Context) -> ArcHandle<wgpu::ShaderModule> {
        self.build(ctx)
    }

    fn reload(&mut self, ctx: &mut Context, data: Vec<u8>) {
        let source = String::from_utf8(data).expect("could not convert to string");

        // validation (native)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let debug_builder = self.clone().source(source.clone());
            if let Err(err) = debug_builder.build_err(ctx) {
                tracing::error!("could not reload shader: {}", err);
                return;
            }
        }

        self.source = source;
    }
}

pub struct PixelCache {
    default_textures: HashMap<[u8; 4], asset::AssetHandle<Image>>,
}

impl PixelCache {
    pub fn new() -> Self {
        Self {
            default_textures: HashMap::new(),
        }
    }

    pub fn allocate(&mut self, ctx: &mut Context, value: [u8; 4]) -> asset::AssetHandle<Image> {
        match self.default_textures.get(&value) {
            Some(handle) => handle.clone(),
            None => {
                let image = Image {
                    texture: render::TextureBuilder::new(render::TextureSource::Data(
                        1,
                        1,
                        value.to_vec(),
                    )),
                    sampler: render::SamplerBuilder::new()
                        .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
                };
                let handle = asset::AssetBuilder::insert(image).build(ctx);
                self.default_textures.insert(value, handle.clone());
                handle
            }
        }
    }
}
