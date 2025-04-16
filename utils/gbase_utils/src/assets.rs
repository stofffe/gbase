use crate::{GpuMesh, Image, Mesh};
use gbase::{
    log,
    pollster::FutureExt,
    render::{self, ArcHandle, ArcShaderModule, ShaderBuilder, TextureWithView},
    wgpu::{self},
    Context,
};
use std::{
    collections::HashMap,
    fs,
    marker::PhantomData,
    sync::{atomic::AtomicU64, Arc},
    time::SystemTime,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

// TODO: should have type aswell
#[derive(PartialOrd, Ord, Debug)]
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

struct ReloadHandle<T: 'static> {
    path: String,
    modified: SystemTime,
    handle: AssetHandle<T>,
}

pub struct Assets {
    meshes: HashMap<AssetHandle<Mesh>, Mesh>,
    shaders: HashMap<AssetHandle<ShaderBuilder>, (ShaderBuilder, bool)>,
    images: HashMap<AssetHandle<Image>, Image>,

    meshes_gpu: HashMap<AssetHandle<Mesh>, (Arc<GpuMesh>, bool)>,
    shaders_gpu: HashMap<AssetHandle<ShaderBuilder>, ArcShaderModule>,
    images_gpu: HashMap<AssetHandle<Image>, (Arc<TextureWithView>, bool)>,

    images_reload: Vec<ReloadHandle<Image>>,
    shaders_reload: Vec<ReloadHandle<ShaderBuilder>>,

    default_images: HashMap<[u8; 4], AssetHandle<Image>>,
}

impl Assets {
    #![allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            meshes_gpu: HashMap::new(),

            images: HashMap::new(),
            images_gpu: HashMap::new(),
            images_reload: Vec::new(),

            default_images: HashMap::new(),

            shaders: HashMap::new(),
            shaders_gpu: HashMap::new(),
            shaders_reload: Vec::new(),
        }
    }

    //
    // Shader
    //

    pub fn allocate_shader_data(&mut self, shader: ShaderBuilder) -> AssetHandle<ShaderBuilder> {
        let handle = AssetHandle::new();
        self.shaders.insert(handle.clone(), (shader, true));
        handle
    }

    pub fn get_shader(&self, id: AssetHandle<ShaderBuilder>) -> &ShaderBuilder {
        &self.shaders.get(&id).unwrap().0 // fine to unwrap here it think
    }
    pub fn get_shader_mut(&mut self, id: AssetHandle<ShaderBuilder>) -> &mut ShaderBuilder {
        let (shader, changed) = self.shaders.get_mut(&id).unwrap();
        *changed = true;

        shader
    }

    pub fn get_shader_gpu(
        &mut self,
        ctx: &mut Context,
        id: AssetHandle<ShaderBuilder>,
    ) -> Result<ArcShaderModule, ()> {
        debug_assert!(self.shaders.contains_key(&id), "handle doesnt exist");

        let (shader, changed) = self.shaders.get_mut(&id).unwrap();
        let shader_gpu = self.shaders_gpu.get_mut(&id);

        if !*changed {
            // log::error!("RETURN");
            return match shader_gpu {
                Some(shader_gpu) => Ok(shader_gpu.clone()),
                None => Err(()),
            };
        }

        *changed = false;

        let Ok(new_gpu_shader) = shader.clone().build_err(ctx) else {
            self.shaders_gpu.remove(&id); // TODO: why is this needed
            return Err(());
        };

        match self.shaders_gpu.get_mut(&id) {
            None => {
                self.shaders_gpu.insert(id.clone(), new_gpu_shader);
                log::info!("create shader gpu buffer");
            }
            Some(gpu_shader) => {
                *gpu_shader = new_gpu_shader;
                log::info!("Update shader gpu buffer");
            }
        }

        Ok(self.shaders_gpu.get(&id).expect("should exst").clone())
    }

    pub fn watch_shader(&mut self, path: String, handle: AssetHandle<ShaderBuilder>) {
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        self.shaders_reload.push(ReloadHandle {
            path,
            modified,
            handle,
        });
    }

    pub fn check_watch_shaders(&mut self) {
        for i in 0..self.shaders_reload.len() {
            let Ok(md) = fs::metadata(&self.shaders_reload[i].path) else {
                continue;
            };

            let modified = md.modified().unwrap();
            if modified != self.shaders_reload[i].modified {
                self.shaders_reload[i].modified = modified;

                let txt = fs::read_to_string(&self.shaders_reload[i].path).unwrap();

                let shader = self.get_shader_mut(self.shaders_reload[i].handle.clone());
                shader.source = txt;
            }
        }
    }

    //
    // Mesh
    //

    // TODO: add shader

    pub fn allocate_mesh_data(&mut self, mesh: Mesh) -> AssetHandle<Mesh> {
        let handle = AssetHandle::new();
        self.meshes.insert(handle.clone(), mesh);
        handle
    }

    pub fn get_mesh(&self, id: AssetHandle<Mesh>) -> &Mesh {
        self.meshes.get(&id).unwrap()
    }
    pub fn get_mesh_mut(&mut self, id: AssetHandle<Mesh>) -> &mut Mesh {
        let (_, gpu_changed) = self.meshes_gpu.get_mut(&id).unwrap();
        *gpu_changed = true;

        self.meshes.get_mut(&id).unwrap()
    }

    pub fn get_mesh_gpu(&mut self, ctx: &Context, id: AssetHandle<Mesh>) -> Arc<GpuMesh> {
        debug_assert!(self.meshes.contains_key(&id), "handle doesnt exist");

        // if handle doesnt exist => create new
        match self.meshes_gpu.get_mut(&id) {
            // create buffer
            None => {
                let mesh = self.meshes.get(&id).expect("handle doesnt exist");
                let gpu_mesh = GpuMesh::new(ctx, mesh);

                self.meshes_gpu.insert(id.clone(), (gpu_mesh.into(), false));
                log::info!("create mesh gpu buffer");
            }
            Some((gpu_mesh, changed)) => {
                if *changed {
                    let mesh = self.meshes.get(&id).expect("handle doesnt exist");
                    *gpu_mesh = GpuMesh::new(ctx, mesh).into();
                    *changed = false;
                    log::info!("Update mesh gpu buffer");
                }
            }
        }

        self.meshes_gpu.get(&id).expect("should exst").0.clone()
    }

    //
    // Image
    //

    pub fn allocate_image_or_default(
        &mut self,
        image: Option<Image>,
        default: [u8; 4],
    ) -> AssetHandle<Image> {
        match image {
            Some(image) => {
                let handle = AssetHandle::new();
                self.images.insert(handle.clone(), image);
                handle
            }
            None => self.allocate_image_pixel(default),
        }
    }

    pub fn allocate_image_pixel(&mut self, pixel: [u8; 4]) -> AssetHandle<Image> {
        match self.default_images.get(&pixel) {
            Some(handle) => handle.clone(),
            None => {
                log::info!("CACHE MISS FOR {:?}", pixel);
                let texture =
                    render::TextureBuilder::new(render::TextureSource::Data(1, 1, pixel.into()));
                let sampler = render::SamplerBuilder::new()
                    .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest);
                let image = Image { texture, sampler };

                let handle = AssetHandle::new();

                self.images.insert(handle.clone(), image);
                self.default_images.insert(pixel, handle.clone());

                handle
            }
        }
    }

    pub fn allocate_image_data(&mut self, image: Image) -> AssetHandle<Image> {
        let handle = AssetHandle::new();
        self.images.insert(handle.clone(), image);
        handle
    }

    pub fn get_image(&self, id: AssetHandle<Image>) -> &Image {
        self.images.get(&id).unwrap()
    }
    pub fn get_image_mut(&mut self, id: AssetHandle<Image>) -> &mut Image {
        if let Some((_, changed)) = self.images_gpu.get_mut(&id) {
            *changed = true;
        }

        self.images.get_mut(&id).unwrap()
    }

    pub fn get_image_gpu(
        &mut self,
        ctx: &mut Context,
        id: AssetHandle<Image>,
    ) -> Arc<TextureWithView> {
        debug_assert!(self.images.contains_key(&id), "handle doesnt exist");

        // if handle doesnt exist => create new
        match self.images_gpu.get_mut(&id) {
            // create buffer
            None => {
                let image = self.images.get(&id).expect("handle doesnt exist");
                let texture = image.texture.clone().build(ctx);
                let sampler = image.sampler.clone().build(ctx);
                let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
                let image_gpu = render::TextureWithView::new(texture, view, sampler);
                self.images_gpu
                    .insert(id.clone(), (image_gpu.into(), false));
            }
            // update buffer
            Some((gpu_image, changed)) => {
                if *changed {
                    let image = self.images.get(&id).expect("handle doesnt exist");
                    let texture = image.texture.clone().build(ctx);
                    let sampler = image.sampler.clone().build(ctx);
                    let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
                    *gpu_image = render::TextureWithView::new(texture, view, sampler).into();
                    *changed = false;
                }
            }
        }

        self.images_gpu.get(&id).expect("should exst").0.clone()
    }

    pub fn watch_image(&mut self, path: String, handle: AssetHandle<Image>) {
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        self.images_reload.push(ReloadHandle {
            path,
            modified,
            handle,
        });
    }

    pub fn check_watch_images(&mut self) {
        for i in 0..self.images_reload.len() {
            let Ok(md) = fs::metadata(&self.images_reload[i].path) else {
                continue;
            };

            let modified = md.modified().unwrap();
            if modified != self.images_reload[i].modified {
                self.images_reload[i].modified = modified;

                let bytes = fs::read(&self.images_reload[i].path).unwrap();

                // TODO: should this even do this here
                // or should it be done in get_gpu
                match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        let img = img.to_rgba8();
                        let image = self.get_image_mut(self.images_reload[i].handle.clone());
                        image.texture.source =
                            render::TextureSource::Data(img.width(), img.height(), img.to_vec());
                    }
                    Err(err) => {
                        log::error!("error loading {:?}: {:?}", self.images_reload[i].path, err);
                    }
                }
            }
        }
    }
}

//
// Generic asset manager
//

pub struct ShaderDescriptor {
    pub label: Option<String>,
    pub source: String,
}

pub struct AssetCache<T: 'static, G: 'static> {
    cpu_cache: HashMap<AssetHandle<T>, (T, bool)>,
    gpu_cache: HashMap<AssetHandle<T>, ArcHandle<G>>,
    reload: Vec<ReloadHandle<T>>,
}

// trait which takes hot reloadable (bytes -> update struct)

pub trait Asset<T: 'static, G: 'static> {
    fn convert(&self, ctx: &mut Context) -> ArcHandle<G>;
    fn reload(&mut self, ctx: &mut Context, data: Vec<u8>);
}

impl Asset<ShaderDescriptor, wgpu::ShaderModule> for ShaderDescriptor {
    // TODO: maybe async? at least for reloading
    fn convert(&self, ctx: &mut Context) -> ArcHandle<wgpu::ShaderModule> {
        let device = render::device(ctx);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(self.source.clone().into()), // TODO: clone here?
        });

        ArcHandle::new(module)
    }

    fn reload(&mut self, ctx: &mut Context, data: Vec<u8>) {
        let source = String::from_utf8(data).expect("could not convert to string");

        // validation
        let device = render::device(ctx);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _ = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(source.clone().into()), // TODO: dont clone here?
        });
        let result = device.pop_error_scope().block_on(); // async, doesnt work for wasm
        if let Some(err) = result {
            log::error!("{:?}", err.to_string());
            return;
        }

        // reload
        self.source = source;
    }
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

    pub fn allocate_reload(&mut self, data: T, reload_path: String) -> AssetHandle<T> {
        let handle = AssetHandle::new();

        self.cpu_cache.insert(handle.clone(), (data, true));
        self.watch(reload_path, handle.clone());

        handle
    }

    pub fn allocate(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::new();

        self.cpu_cache.insert(handle.clone(), (data, true));

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
    pub fn get_gpu(&mut self, ctx: &mut Context, id: AssetHandle<T>) -> Result<ArcHandle<G>, ()> {
        debug_assert!(self.cpu_cache.contains_key(&id), "handle doesnt exist");

        match self.gpu_cache.get_mut(&id) {
            // create buffer
            None => {
                let (cpu_typ, changed) = self.cpu_cache.get(&id).expect("handle doesnt exist");
                debug_assert!(*changed);

                let gpu_typ = cpu_typ.convert(ctx);
                self.gpu_cache.insert(id.clone(), gpu_typ);
                log::info!("create type gpu buffer");
            }
            // get cached or update buffer
            Some(gpu_typ) => {
                let (cpu_typ, changed) = self.cpu_cache.get_mut(&id).expect("handle doesnt exist");
                if *changed {
                    *changed = false;
                    *gpu_typ = cpu_typ.convert(ctx);
                    log::info!("Update mesh gpu buffer");
                }
            }
        }

        Ok(self.gpu_cache.get_mut(&id).expect("should exist").clone())
    }

    pub fn watch(&mut self, path: String, handle: AssetHandle<T>) {
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        self.reload.push(ReloadHandle {
            path,
            modified,
            handle,
        });
    }

    pub fn check_watch(&mut self, ctx: &mut Context) {
        for i in 0..self.reload.len() {
            let Ok(md) = fs::metadata(&self.reload[i].path) else {
                log::warn!("could not get metadata");
                continue;
            };

            let modified = md.modified().expect("could not get modified");
            if modified != self.reload[i].modified {
                self.reload[i].modified = modified;

                let bytes = fs::read(&self.reload[i].path).unwrap();
                self.get_mut(self.reload[i].handle.clone())
                    .reload(ctx, bytes);
            }
        }
    }
}
