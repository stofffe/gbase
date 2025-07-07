use super::{
    Asset, AssetHandle, ConvertableRenderAsset, DynAsset, DynAssetHandle, DynAssetLoadFn,
    DynAssetOnLoadFn, DynAssetWriteFn, DynRenderAsset, LoadableAsset, TypedAssetOnLoadFn,
    WriteableAsset,
};
use crate::{render::ArcHandle, Context};
use futures_channel::mpsc;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

// TODO: maybe create new types for the complicated ones

pub type RenderAssetKey = (DynAssetHandle, TypeId);

pub struct AssetCache {
    cache: HashMap<DynAssetHandle, DynAsset>,

    render_cache: HashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_last_valid: HashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_invalidate_lookup: HashMap<DynAssetHandle, HashSet<TypeId>>,

    // async loading
    load_sender: mpsc::UnboundedSender<(DynAssetHandle, DynAsset)>,
    load_receiver: mpsc::UnboundedReceiver<(DynAssetHandle, DynAsset)>,
    currently_loading: HashSet<DynAssetHandle>,
    reload_on_load: HashMap<DynAssetHandle, DynAssetOnLoadFn>,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) ext: AssetCacheExt,
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
            render_cache_last_valid: HashMap::new(),
            render_cache_invalidate_lookup: HashMap::new(),

            currently_loading: HashSet::new(),
            load_sender,
            load_receiver,
            reload_on_load: HashMap::new(),

            #[cfg(not(target_arch = "wasm32"))]
            ext: AssetCacheExt {
                reload_handles: HashMap::new(),
                reload_functions: HashMap::new(),
                reload_watcher,
                reload_receiver,

                write_handles: HashMap::new(),
                write_functions: HashMap::new(),
                write_dirty: HashSet::new(),
            },
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
        invalidate_render_cache(
            &mut self.render_cache,
            &self.render_cache_invalidate_lookup,
            handle.as_any(),
        );

        // set dirty
        // TODO: move inside
        #[cfg(not(target_arch = "wasm32"))]
        self.ext.write_dirty.insert(handle.clone().as_any());

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

    pub fn load<T: Asset + LoadableAsset>(
        &mut self,
        handle: AssetHandle<T>,
        path: &Path,
        on_load: Option<TypedAssetOnLoadFn<T>>,
    ) -> AssetHandle<T> {
        let path = path.to_path_buf();

        if let Some(on_load) = on_load {
            // Wrap the callback to accept DynAsset and downcast internally
            let wrapped_callback: Box<dyn Fn(&mut DynAsset)> = Box::new(move |dyn_asset| {
                // Downcast DynAsset to the concrete type T
                let asset = (dyn_asset.as_mut() as &mut dyn Any)
                    .downcast_mut::<T>()
                    .expect("Failed to downcast DynAsset to T in on_load callback");
                on_load(asset);
            });

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

    //
    // Render assets
    //

    pub fn convert<G: ConvertableRenderAsset>(
        &mut self,
        ctx: &mut Context,
        handle: AssetHandle<G::SourceAsset>,
        params: &G::Params,
    ) -> Option<ArcHandle<G>> {
        let Some(source_asset) = self.get(handle.clone()) else {
            tracing::warn!("could not get source asset");
            return None;
        };

        let key = (handle.clone().as_any(), TypeId::of::<G>());
        let render_asset_exists = self.render_cache.contains_key(&key);
        if !render_asset_exists {
            match G::convert(ctx, source_asset, params) {
                Ok(render_asset) => {
                    let render_asset_handle = ArcHandle::new(render_asset).upcast();
                    // actual cache
                    self.render_cache
                        .insert(key.clone(), render_asset_handle.clone());
                    // last valid cache
                    self.render_cache_last_valid
                        .insert(key.clone(), render_asset_handle);
                    // invalidate lookup
                    self.render_cache_invalidate_lookup
                        .entry(handle.clone().as_any())
                        .or_default()
                        .insert(TypeId::of::<G>());
                }
                Err(err) => {
                    tracing::warn!("could not convert {}", err);
                }
            };
        }

        self.render_cache
            .get(&key)
            .map(|a| a.downcast::<G>().expect("could not downcast"))
            .or_else(|| {
                // try last valid
                self.render_cache_last_valid
                    .get(&key)
                    .map(|a| a.downcast::<G>().expect("could not downcast"))
            })
    }

    //
    // Polling
    //

    pub fn poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.ext.poll_reload(
                &mut self.cache,
                &mut self.render_cache,
                &self.render_cache_invalidate_lookup,
                &self.reload_on_load,
            );
            self.ext.poll_write(&mut self.cache);
        }

        self.poll_loaded();
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(&mut self) {
        while let Ok(Some((handle, mut asset))) = self.load_receiver.try_next() {
            // callback first
            if let Some(on_load) = self.reload_on_load.get(&handle) {
                on_load(&mut asset);
            }

            // insert in cache
            self.cache.insert(handle.clone(), asset);

            // remove from currently loaded
            self.currently_loading.remove(&handle.as_any());

            // invalidate render cache
            invalidate_render_cache(
                &mut self.render_cache,
                &self.render_cache_invalidate_lookup,
                handle,
            );
        }
    }

    pub fn all_loaded(&self) -> bool {
        self.currently_loading.is_empty()
    }

    pub fn handle_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        !self.currently_loading.contains(&handle.as_any())
    }

    pub fn handles_loaded(&self, handles: impl IntoIterator<Item = DynAssetHandle>) -> bool {
        for handle in handles {
            if !self.currently_loading.contains(&handle) {
                return false;
            }
        }
        true
    }
}

//
// Hot reload extension
//

#[cfg(not(target_arch = "wasm32"))]
pub struct AssetCacheExt {
    // reloading
    reload_handles: HashMap<PathBuf, Vec<DynAssetHandle>>,
    reload_functions: HashMap<TypeId, DynAssetLoadFn>,
    reload_watcher: notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::FsEventWatcher>,
    reload_receiver: mpsc::UnboundedReceiver<PathBuf>,

    // writing
    write_handles: HashMap<DynAssetHandle, PathBuf>,
    write_functions: HashMap<TypeId, DynAssetWriteFn>,
    write_dirty: HashSet<DynAssetHandle>,
}

// TODO: check if canoicalize is necessary

#[cfg(not(target_arch = "wasm32"))]
impl AssetCacheExt {
    /// Register asset for being watched for hot reloads
    #[cfg(not(target_arch = "wasm32"))]
    pub fn watch<T: Asset + LoadableAsset>(&mut self, handle: AssetHandle<T>, path: &Path) {
        // need absolute path since notify uses them
        let absolute_path = fs::canonicalize(path).unwrap();

        // start watching path
        self.reload_watcher
            .watcher()
            .watch(
                &absolute_path,
                notify_debouncer_mini::notify::RecursiveMode::Recursive, // TODO: non recursive?
            )
            .unwrap();

        // map path to handle
        let handles = self.reload_handles.entry(absolute_path).or_default();
        handles.push(handle.as_any());

        // store reload function
        self.reload_functions
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(|path| Box::new(pollster::block_on(T::load(path)))));
    }

    /// Register asset for being written to disk when updated
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write<T: Asset + WriteableAsset>(&mut self, handle: AssetHandle<T>, path: &Path) {
        let path = path.to_path_buf();

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

    // check if any files are scheduled for writing to disk
    pub fn poll_write(&mut self, cache: &mut HashMap<DynAssetHandle, DynAsset>) {
        for handle in self.write_dirty.drain() {
            if let Some(path) = self.write_handles.get(&handle) {
                let asset = cache.get_mut(&handle);

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
    pub fn poll_reload(
        &mut self,
        cache: &mut HashMap<DynAssetHandle, DynAsset>,
        render_cache: &mut HashMap<RenderAssetKey, DynRenderAsset>,
        render_cache_invalidate_lookup: &HashMap<DynAssetHandle, HashSet<TypeId>>,
        on_load: &HashMap<DynAssetHandle, DynAssetOnLoadFn>,
    ) {
        while let Ok(Some(path)) = self.reload_receiver.try_next() {
            println!("1 reload {:?}", path);
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
                    if let Some(on_load) = on_load.get(handle) {
                        on_load(&mut asset);
                    }

                    // insert into cache
                    cache.insert(handle.clone(), asset);

                    // invalidate render cache
                    invalidate_render_cache(
                        render_cache,
                        render_cache_invalidate_lookup,
                        handle.as_any(),
                    );
                }
            }
        }
    }
}

pub fn invalidate_render_cache(
    render_cache: &mut HashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_invalidate_lookup: &HashMap<DynAssetHandle, HashSet<TypeId>>,
    handle: DynAssetHandle,
) {
    if let Some(render_types) = render_cache_invalidate_lookup.get(&handle) {
        for render_type in render_types {
            render_cache.remove(&(handle.clone(), *render_type));
        }
    }
}

impl<T: Asset + 'static> AssetHandle<T> {
    pub fn loaded(self, cache: &AssetCache) -> bool {
        cache.handle_loaded(self)
    }
    pub fn get(self, cache: &mut AssetCache) -> Option<&T> {
        cache.get(self.clone())
    }
    pub fn get_mut(self, cache: &mut AssetCache) -> Option<&mut T> {
        cache.get_mut(self.clone())
    }
    pub fn convert<G: ConvertableRenderAsset<SourceAsset = T>>(
        self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        params: &G::Params,
    ) -> Option<ArcHandle<G>> {
        cache.convert(ctx, self, params)
    }
}
