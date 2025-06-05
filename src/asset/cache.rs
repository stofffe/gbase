use crate::render::{self, ArcHandle};
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use super::{
    Asset, AssetHandle, ConvertableRenderAsset, DynAsset, DynAssetLoadFn, DynAssetWriteFn,
    DynRenderAsset, LoadableAsset, WriteableAsset,
};

pub struct AssetCache {
    cache: HashMap<AssetHandle<DynAsset>, DynAsset>,
    render_cache: HashMap<AssetHandle<DynAsset>, DynRenderAsset>,

    load_handles: HashMap<AssetHandle<DynAsset>, PathBuf>,
    load_dirty: HashSet<AssetHandle<DynAsset>>,

    // async loading
    load_sender: mpsc::Sender<(AssetHandle<DynAsset>, DynAsset)>,
    load_receiver: mpsc::Receiver<(AssetHandle<DynAsset>, DynAsset)>,

    // reloading
    reload_functions: HashMap<TypeId, DynAssetLoadFn>,
    reload_handles: HashMap<PathBuf, Vec<AssetHandle<DynAsset>>>,
    reload_watcher: notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::FsEventWatcher>,
    reload_receiver: mpsc::Receiver<PathBuf>,
    reload_sender: mpsc::Sender<PathBuf>,

    // writing
    write_functions: HashMap<TypeId, DynAssetWriteFn>,
}

impl AssetCache {
    pub fn new() -> Self {
        let (reload_sender, reload_receiver) = mpsc::channel();
        let (loaded_sender, loaded_receiver) = mpsc::channel();
        let sender_copy = reload_sender.clone();

        let reload_watcher = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(100),
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        sender_copy
                            .clone()
                            .send(event.path)
                            .expect("could not send");
                    }
                }
                Err(err) => println!("debounced result error: {}", err),
            },
        )
        .expect("could not create watcher");

        Self {
            cache: HashMap::new(),
            render_cache: HashMap::new(),
            load_dirty: HashSet::new(),
            reload_handles: HashMap::new(),
            load_handles: HashMap::new(),

            write_functions: HashMap::new(),

            reload_functions: HashMap::new(),
            reload_receiver,
            reload_sender,
            reload_watcher,

            load_sender: loaded_sender,
            load_receiver: loaded_receiver,
        }
    }

    //
    // Assets
    //

    pub fn insert<T: Asset + 'static>(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new();
        self.cache.insert(handle.clone().as_any(), Box::new(data));
        handle
    }

    // TODO: add get_or_default (e.g. 1x1 white pixel for image)
    //
    // could return error union [Ok, Invalid, Loading]
    pub fn get<T: Asset + 'static>(&self, handle: AssetHandle<T>) -> Option<&T> {
        self.cache.get(&handle.as_any()).map(|asset| {
            (asset.as_ref() as &dyn Any)
                .downcast_ref::<T>()
                .expect("could not downcast")
        })
    }

    pub fn get_mut<T: Asset + 'static>(&mut self, handle: AssetHandle<T>) -> Option<&mut T> {
        // invalidate gpu cache
        self.render_cache.remove(&handle.clone().as_any());

        // set dirty
        self.load_dirty.insert(handle.clone().as_any());

        // get value and convert to T
        self.cache.get_mut(&handle.as_any()).map(|asset| {
            (asset.as_mut() as &mut dyn Any)
                .downcast_mut::<T>()
                .expect("could not downcast")
        })
    }

    //
    // Reloading
    //

    // TODO: investigate using watch and write manually main, maybe store path in asset handle also

    /// Load a file
    pub fn load<T: Asset + LoadableAsset>(&mut self, path: &Path, sync: bool) -> AssetHandle<T> {
        let path = fs::canonicalize(path).unwrap();
        let handle = AssetHandle::<T>::new();

        if sync {
            let data = T::load(&path);
            self.cache.insert(handle.clone().as_any(), Box::new(data));
        } else {
            let path_clone = path.clone();
            let handle_clone = handle.clone();
            let loaded_sender_clone = self.load_sender.clone();
            std::thread::spawn(move || {
                let data = T::load(&path_clone);
                loaded_sender_clone
                    .send((handle_clone.as_any(), Box::new(data)))
                    .expect("could not send");
            });
        }

        handle
    }

    /// Load a file
    ///
    /// Register asset for being watched for hot reloads
    pub fn load_watch<T: Asset + LoadableAsset>(
        &mut self,
        path: &Path,
        sync: bool,
    ) -> AssetHandle<T> {
        let handle = self.load(path, sync);
        self.watch(handle.clone(), path);
        handle
    }

    /// Load a file
    ///
    /// Register asset for being written to disk when updated
    pub fn load_write<T: Asset + LoadableAsset + WriteableAsset>(
        &mut self,
        path: &Path,
        sync: bool,
    ) -> AssetHandle<T> {
        let handle = self.load(path, sync);
        self.write(handle.clone(), path);
        handle
    }
    /// Load a file
    ///
    /// Register asset for being watched for hot reloads
    /// Register asset for being written to disk when updated
    pub fn load_watch_write<T: Asset + LoadableAsset + WriteableAsset>(
        &mut self,
        path: &Path,
        sync: bool,
    ) -> AssetHandle<T> {
        let handle = self.load(path, sync);
        self.watch(handle.clone(), path);
        self.write(handle.clone(), path);
        handle
    }

    /// Register asset for being watched for hot reloads
    pub fn watch<T: Asset + LoadableAsset>(&mut self, handle: AssetHandle<T>, path: &Path) {
        let path = fs::canonicalize(path).unwrap();

        // start watching path
        self.reload_watcher
            .watcher()
            .watch(
                &path,
                notify_debouncer_mini::notify::RecursiveMode::Recursive,
            )
            .unwrap();

        // map path to handle
        let handles = self.reload_handles.entry(path).or_default();
        handles.push(handle.as_any());

        // store reload function
        self.reload_functions
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(|path| Box::new(T::load(path))));
    }

    /// Register asset for being written to disk when updated
    pub fn write<T: Asset + WriteableAsset>(&mut self, handle: AssetHandle<T>, path: &Path) {
        let path = fs::canonicalize(path).unwrap();
        // map handle to path
        self.load_handles.insert(handle.as_any(), path.clone());

        // store reload function
        self.write_functions
            .entry(TypeId::of::<T>())
            .or_insert_with(|| {
                Box::new(|asset, path| {
                    let typed = (asset.as_mut() as &mut dyn Any)
                        .downcast_mut::<T>()
                        .expect("could not cast during write");
                    typed.write(path);
                })
            });
    }

    //
    // Render assets
    //

    pub fn convert<G: ConvertableRenderAsset>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_cache: &mut render::RenderCache,
        handle: AssetHandle<G::SourceAsset>,
        params: &G::Params,
    ) -> Option<ArcHandle<G>> {
        // create new if not in cache
        if !self.render_cache.contains_key(&handle.clone().as_any()) {
            let asset = self.get(handle.clone());

            if let Some(asset) = asset {
                let converted = G::convert(device, queue, render_cache, asset, params);
                self.render_cache
                    .insert(handle.clone().as_any(), ArcHandle::new(converted).upcast());
            }
        }

        // get value and convert to G
        self.render_cache
            .get(&handle.as_any())
            .map(|a| a.downcast::<G>().expect("could not downcast"))
    }

    //
    // Polling
    //

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(&mut self) {
        for (handle, asset) in self.load_receiver.try_iter() {
            self.cache.insert(handle.clone(), asset);
            self.render_cache.remove(&handle);
        }
    }

    // check if any files are scheduled for writing to disk
    pub fn poll_write(&mut self) {
        for handle in self.load_dirty.drain() {
            if let Some(path) = self.load_handles.get(&handle) {
                let asset = self.cache.get_mut(&handle);

                // write if loaded
                if let Some(asset) = asset {
                    let write_fn = self
                        .write_functions
                        .get(&handle.ty_id)
                        .expect("could not get write fn");

                    write_fn(asset, path);
                }
            }
        }
    }

    // checks if any files changed and spawns a thread which reloads the data
    pub fn poll_reload(&mut self) {
        for path in self.reload_receiver.try_iter() {
            if let Some(handles) = self.reload_handles.get_mut(&path) {
                for handle in handles {
                    println!("reload {:?}", path);

                    // create/overwrite current value
                    let loader_fn = self
                        .reload_functions
                        .get(&handle.ty_id)
                        .expect("could not get loader fn");
                    let asset = loader_fn(&path);
                    self.cache.insert(handle.clone(), asset);

                    // invalidate render cache
                    self.render_cache.remove(handle);
                }
            }
        }
    }

    pub fn force_reload(&self, path: PathBuf) {
        self.reload_sender.send(path).expect("could not send path");
    }
}
