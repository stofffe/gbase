use crate::{texture_builder_from_image_bytes, GpuMesh, Image, Mesh};
use gbase::{
    log,
    notify::{self, Watcher},
    render::{self, TextureWithView},
    wgpu,
    winit::platform::modifier_supplement,
    Context,
};
use gltf::json::Path;
use image::GenericImageView;
use std::{
    collections::HashMap,
    fs,
    marker::PhantomData,
    path::PathBuf,
    sync::{atomic::AtomicU64, mpsc, Arc},
    time::SystemTime,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

// TODO: should have type aswell
#[derive(PartialOrd, Ord, Clone, Debug)]
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

struct ReloadHandle<T: 'static> {
    path: String,
    modified: SystemTime,
    handle: AssetHandle<T>,
}

pub struct Assets {
    meshes: HashMap<AssetHandle<Mesh>, Mesh>,
    meshes_gpu: HashMap<AssetHandle<Mesh>, (Arc<GpuMesh>, bool)>,

    images: HashMap<AssetHandle<Image>, Image>,
    images_gpu: HashMap<AssetHandle<Image>, (Arc<TextureWithView>, bool)>,

    images_reload: Vec<ReloadHandle<Image>>,

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
        }
    }

    //
    // Mesh
    //

    // pub fn allocate_mesh(&mut self) -> AssetHandle<Mesh> {
    //     let handle = AssetHandle::new();
    //     self.meshes.insert(handle.clone(), Mesh::default());
    //     handle
    // }
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
