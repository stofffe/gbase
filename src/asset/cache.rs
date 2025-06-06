use futures_channel::mpsc;

use crate::render::{self, ArcHandle};
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    Asset, AssetHandle, ConvertableRenderAsset, DynAsset, DynAssetLoadFn, DynAssetOnLoadFn,
    DynAssetWriteFn, DynRenderAsset, LoadableAsset, TypedAssetOnLoadFn, WriteableAsset,
};

pub struct AssetCache {
    cache: HashMap<AssetHandle<DynAsset>, DynAsset>,
    render_cache: HashMap<AssetHandle<DynAsset>, DynRenderAsset>,

    // async loading
    load_sender: mpsc::UnboundedSender<(AssetHandle<DynAsset>, DynAsset)>,
    load_receiver: mpsc::UnboundedReceiver<(AssetHandle<DynAsset>, DynAsset)>,
    currently_loading: HashSet<AssetHandle<DynAsset>>,

    // convert
    convert_last_valid: HashMap<AssetHandle<DynAsset>, DynRenderAsset>,

    // reloading
    #[cfg(not(target_arch = "wasm32"))]
    reload_handles: HashMap<PathBuf, Vec<AssetHandle<DynAsset>>>,
    #[cfg(not(target_arch = "wasm32"))]
    reload_functions: HashMap<TypeId, DynAssetLoadFn>,
    #[cfg(not(target_arch = "wasm32"))]
    reload_watcher: notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::FsEventWatcher>,
    #[cfg(not(target_arch = "wasm32"))]
    reload_receiver: mpsc::UnboundedReceiver<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    reload_on_load: HashMap<AssetHandle<DynAsset>, DynAssetOnLoadFn>,

    // writing
    #[cfg(not(target_arch = "wasm32"))]
    write_handles: HashMap<AssetHandle<DynAsset>, PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    write_functions: HashMap<TypeId, DynAssetWriteFn>,
    #[cfg(not(target_arch = "wasm32"))]
    write_dirty: HashSet<AssetHandle<DynAsset>>,
}

impl AssetCache {
    pub fn new() -> Self {
        let (load_sender, load_receiver) = futures_channel::mpsc::unbounded();

        #[cfg(not(target_arch = "wasm32"))]
        let (reload_watcher, reload_receiver) = {
            let (reload_sender, reload_receiver) = mpsc::unbounded();
            let sender_copy = reload_sender.clone();
            let reload_watcher = notify_debouncer_mini::new_debouncer(
                Duration::from_millis(100),
                move |res: notify_debouncer_mini::DebounceEventResult| match res {
                    Ok(events) => {
                        for event in events {
                            sender_copy
                                .clone()
                                .unbounded_send(event.path)
                                .expect("could not send");
                        }
                    }
                    Err(err) => println!("debounced result error: {}", err),
                },
            )
            .expect("could not create watcher");
            (reload_watcher, reload_receiver)
        };

        Self {
            cache: HashMap::new(),
            render_cache: HashMap::new(),

            currently_loading: HashSet::new(),
            load_sender,
            load_receiver,
            convert_last_valid: HashMap::new(),

            #[cfg(not(target_arch = "wasm32"))]
            reload_handles: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            reload_functions: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            reload_watcher,
            #[cfg(not(target_arch = "wasm32"))]
            reload_receiver,
            #[cfg(not(target_arch = "wasm32"))]
            reload_on_load: HashMap::new(),

            #[cfg(not(target_arch = "wasm32"))]
            write_handles: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            write_functions: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            write_dirty: HashSet::new(),
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
        #[cfg(not(target_arch = "wasm32"))]
        self.write_dirty.insert(handle.clone().as_any());

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

    // TODO: use loader api for reloading to make it simpler?
    // TODO: investigate using watch and write manually main, maybe store path in asset handle also
    pub fn load<T: Asset + LoadableAsset>(
        &mut self,
        handle: AssetHandle<T>,
        path: &Path,
        on_load: Option<TypedAssetOnLoadFn<T>>,
    ) -> AssetHandle<T> {
        let path = path.to_path_buf();
        #[cfg(not(target_arch = "wasm32"))]
        let path = fs::canonicalize(path).unwrap();

        if let Some(on_load) = on_load {
            // Wrap the callback to accept DynAsset and downcast internally
            let wrapped_callback: Box<dyn Fn(&mut DynAsset)> = Box::new(move |dyn_asset| {
                // Downcast DynAsset to the concrete type T
                let asset = (dyn_asset.as_mut() as &mut dyn Any)
                    .downcast_mut::<T>()
                    .expect("Failed to downcast DynAsset to T in on_load callback");
                on_load(asset);
            });

            #[cfg(not(target_arch = "wasm32"))]
            self.reload_on_load
                .insert(handle.clone().as_any(), wrapped_callback);
        }

        // add to currently loading
        self.currently_loading.insert(handle.as_any());

        let path_clone = path.clone();
        let handle_clone = handle.clone();
        let loaded_sender_clone = self.load_sender.clone();

        // load async
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            pollster::block_on(async {
                let data = T::load(&path_clone).await;
                loaded_sender_clone
                    .unbounded_send((handle_clone.as_any(), Box::new(data)))
                    .expect("could not send");
            })
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let data = T::load(&path_clone).await;
            loaded_sender_clone
                .unbounded_send((handle_clone.as_any(), Box::new(data)))
                .expect("could not send");
        });

        handle
    }

    /// Register asset for being watched for hot reloads
    #[cfg(not(target_arch = "wasm32"))]
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
            .or_insert_with(|| Box::new(|path| Box::new(pollster::block_on(T::load(path)))));
    }

    /// Register asset for being written to disk when updated
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write<T: Asset + WriteableAsset>(&mut self, handle: AssetHandle<T>, path: &Path) {
        let path = fs::canonicalize(path).unwrap();

        // map handle to path
        self.write_handles.insert(handle.as_any(), path.clone());

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

    // TODO: complete chaos
    pub fn convert<G: ConvertableRenderAsset>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_cache: &mut render::RenderCache,
        handle: AssetHandle<G::SourceAsset>,
        params: &G::Params,
    ) -> Option<ArcHandle<G>> {
        let Some(source_asset) = self.get(handle.clone()) else {
            return None;
        };

        let render_asset_exists = self.render_cache.contains_key(&handle.clone().as_any());
        if !render_asset_exists {
            match G::convert(device, queue, render_cache, source_asset, params) {
                Ok(render_asset) => {
                    let render_asset_handle = ArcHandle::new(render_asset).upcast();
                    self.render_cache
                        .insert(handle.clone().as_any(), render_asset_handle.clone());
                    self.convert_last_valid
                        .insert(handle.as_any(), render_asset_handle);
                }
                Err(err) => {
                    tracing::warn!("could not convert {}", err);
                }
            };
        }

        // try current value
        // todo!()
        self.render_cache
            .get(&handle.as_any())
            .map(|a| a.downcast::<G>().expect("could not downcast"))
            .or_else(|| {
                // try last valid
                self.convert_last_valid
                    .get(&handle.as_any())
                    .map(|a| a.downcast::<G>().expect("could not downcast"))
            })
    }

    //
    // Polling
    //

    pub fn poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_reload();
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_write();
        self.poll_loaded();
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(&mut self) {
        while let Ok(Some((handle, mut asset))) = self.load_receiver.try_next() {
            // callback first
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(on_load) = self.reload_on_load.get(&handle) {
                on_load(&mut asset);
            }

            // insert in cache
            self.cache.insert(handle.clone(), asset);

            // remove from currently loaded
            self.currently_loading.remove(&handle.as_any());

            // invalidate render cache
            self.render_cache.remove(&handle);
        }
    }

    // check if any files are scheduled for writing to disk
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll_write(&mut self) {
        for handle in self.write_dirty.drain() {
            if let Some(path) = self.write_handles.get(&handle) {
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll_reload(&mut self) {
        while let Ok(Some(path)) = self.reload_receiver.try_next() {
            if let Some(handles) = self.reload_handles.get_mut(&path) {
                for handle in handles {
                    println!("reload {:?}", path);

                    // load new fn
                    let loader_fn = self
                        .reload_functions
                        .get(&handle.ty_id)
                        .expect("could not get loader fn");
                    let mut asset = loader_fn(&path);

                    // run on load
                    if let Some(on_load) = self.reload_on_load.get(handle) {
                        on_load(&mut asset);
                    }

                    // insert into cache
                    self.cache.insert(handle.clone(), asset);

                    // invalidate render cache
                    self.render_cache.remove(handle);
                }
            }
        }
    }

    pub fn all_loaded(&mut self) -> bool {
        self.currently_loading.is_empty()
    }

    /// Wait for all async assets to be loaded
    pub fn wait_all(&mut self) {
        // while !self.currently_loading.is_empty() {
        //     std::thread::sleep(Duration::from_millis(100));
        //     self.poll_loaded();
        // }
    }

    pub fn wait_for<T: Asset + LoadableAsset>(&mut self, handle: AssetHandle<T>) {
        // while self.currently_loading.contains(&handle.as_any()) {
        //     std::thread::sleep(Duration::from_millis(100));
        //     self.poll_loaded();
        // }
    }
}
