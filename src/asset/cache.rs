use super::{
    Asset, AssetHandle, AssetLoader, AssetWriter, ConvertableRenderAsset, DynAsset, DynAssetHandle,
    DynAssetLoadFn, DynAssetWriteFn, DynRenderAsset,
};
use crate::{render::ArcHandle, Context};
use futures_channel::mpsc;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

pub type RenderAssetKey = (DynAssetHandle, TypeId);
pub struct AssetCache {
    cache: FxHashMap<DynAssetHandle, DynAsset>,
    just_loaded: FxHashSet<DynAssetHandle>,
    render_cache: FxHashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_last_valid: FxHashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_invalidate_lookup: FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,

    // async loading
    load_sender: mpsc::UnboundedSender<(DynAssetHandle, DynAsset)>,
    load_receiver: mpsc::UnboundedReceiver<(DynAssetHandle, DynAsset)>,
    currently_loading: FxHashSet<DynAssetHandle>,
    load_ctx: LoadContext,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) ext: AssetCacheExt,
    // specialized caches
    // pixel textures
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

        let load_ctx = LoadContext {
            sender: load_sender.clone(),
        };

        Self {
            load_ctx: load_ctx.clone(),
            just_loaded: FxHashSet::default(),

            cache: FxHashMap::default(),
            render_cache: FxHashMap::default(),
            render_cache_last_valid: FxHashMap::default(),
            render_cache_invalidate_lookup: FxHashMap::default(),

            currently_loading: FxHashSet::default(),
            load_sender,
            load_receiver,

            #[cfg(not(target_arch = "wasm32"))]
            ext: AssetCacheExt {
                handle_to_type: FxHashMap::default(),

                reload_handles: FxHashMap::default(),
                reload_functions: FxHashMap::default(),
                reload_watcher,
                reload_receiver,

                write_handles: FxHashMap::default(),
                write_functions: FxHashMap::default(),
                write_dirty: FxHashSet::default(),
                load_ctx,
            },
        }
    }

    pub fn load_context(&self) -> &LoadContext {
        &self.load_ctx
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

    pub fn load<T: AssetLoader + Send + Sync + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        path: &Path,
        loader: T,
    ) -> AssetHandle<T::Asset> {
        let path = path.to_path_buf();

        // add to currently loading
        self.currently_loading.insert(handle.as_any());

        let path_clone = path.clone();
        let handle_clone = handle;
        let loaded_sender_clone = self.load_sender.clone();
        let load_context = self.load_ctx.clone();

        // load async
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            pollster::block_on(async {
                let data = loader.load(load_context, &path_clone).await;
                // let data = T::load(load_context, &path_clone).await;
                loaded_sender_clone
                    .unbounded_send((handle_clone.as_any(), Box::new(data)))
                    .expect("could not send");
            })
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let data = loader.load(load_context, &path_clone).await;
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
    ) -> Option<ArcHandle<G>> {
        if self.get(handle).is_none() {
            tracing::warn!("could not get source asset");
            return None;
        };

        let key = (handle.clone().as_any(), TypeId::of::<G>());
        let render_asset_exists = self.render_cache.contains_key(&key);
        if !render_asset_exists {
            match G::convert(ctx, self, handle) {
                Ok(render_asset) => {
                    let render_asset_handle = ArcHandle::new(render_asset).upcast();
                    // actual cache
                    self.render_cache.insert(key, render_asset_handle.clone());
                    // last valid cache
                    self.render_cache_last_valid
                        .insert(key, render_asset_handle);
                    // invalidate lookup
                    self.render_cache_invalidate_lookup
                        .entry(handle.as_any())
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
        self.just_loaded.clear();

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.ext.poll_reload(
                &mut self.cache,
                &mut self.render_cache,
                &self.render_cache_invalidate_lookup,
                &mut self.just_loaded,
            );
            self.ext.poll_write(&mut self.cache);
        }

        self.poll_loaded();
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(&mut self) {
        while let Ok(Some((handle, asset))) = self.load_receiver.try_next() {
            // insert in cache
            self.cache.insert(handle, asset);

            // remove from currently loaded
            self.currently_loading.remove(&handle.as_any());

            // invalidate render cache
            invalidate_render_cache(
                &mut self.render_cache,
                &self.render_cache_invalidate_lookup,
                handle,
            );

            //
            self.just_loaded.insert(handle);
        }
    }

    pub fn all_loaded(&self) -> bool {
        self.currently_loading.is_empty()
    }

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.just_loaded.contains(&handle.as_any())
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
// Load context
//

#[derive(Debug, Clone)]
pub struct LoadContext {
    sender: mpsc::UnboundedSender<(DynAssetHandle, DynAsset)>,
}

impl LoadContext {
    pub fn new(sender: mpsc::UnboundedSender<(DynAssetHandle, DynAsset)>) -> Self {
        Self { sender }
    }

    pub fn insert<T: Asset>(&self, value: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new();
        self.sender
            .unbounded_send((handle.as_any(), Box::new(value)))
            .unwrap();
        handle
    }

    // TODO: ref vs owned self?
    // pub async fn load<T: LoadableAsset>(&self, path: impl Into<PathBuf>) -> AssetHandle<T> {
    //     let path = path.into();
    //     let value = T::load(self.clone(), &path).await;
    //     self.insert(value)
    // }
}

//
// Hot reload extension
//

#[cfg(not(target_arch = "wasm32"))]
pub struct AssetCacheExt {
    handle_to_type: FxHashMap<DynAssetHandle, TypeId>,

    // reloading
    reload_handles: FxHashMap<PathBuf, Vec<DynAssetHandle>>,
    reload_functions: FxHashMap<TypeId, DynAssetLoadFn>,
    reload_watcher: notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::FsEventWatcher>,
    reload_receiver: mpsc::UnboundedReceiver<PathBuf>,

    // writing
    write_handles: FxHashMap<DynAssetHandle, PathBuf>,
    write_functions: FxHashMap<TypeId, DynAssetWriteFn>,
    write_dirty: FxHashSet<DynAssetHandle>,

    // load context
    load_ctx: LoadContext,
}

// TODO: check if canoicalize is necessary

#[cfg(not(target_arch = "wasm32"))]
impl AssetCacheExt {
    /// Register asset for being watched for hot reloads
    #[cfg(not(target_arch = "wasm32"))]
    pub fn watch<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        path: &Path,
        loader: T,
    ) {
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

        // map handle to type
        self.handle_to_type
            .insert(handle.as_any(), TypeId::of::<T::Asset>());

        // store reload function
        self.reload_functions
            .entry(TypeId::of::<T::Asset>())
            .or_insert_with(|| {
                Box::new(move |load_ctx, path| {
                    Box::new(pollster::block_on(loader.clone().load(load_ctx, path)))
                })
            });
    }

    /// Register asset for being written to disk when updated
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write<T: AssetWriter>(&mut self, handle: AssetHandle<T::Asset>, path: &Path) {
        let path = path.to_path_buf();

        // map handle to path
        self.write_handles.insert(handle.as_any(), path.clone());

        // map handle to type
        self.handle_to_type
            .insert(handle.as_any(), TypeId::of::<T::Asset>());

        // store reload function
        self.write_functions
            .entry(TypeId::of::<T::Asset>())
            .or_insert_with(|| {
                Box::new(|asset, path| {
                    let typed = (asset.as_mut() as &mut dyn Any)
                        .downcast_mut::<T::Asset>()
                        .expect("could not cast during write");
                    T::write(typed, path);
                })
            });
    }

    // check if any files are scheduled for writing to disk
    pub fn poll_write(&mut self, cache: &mut FxHashMap<DynAssetHandle, DynAsset>) {
        for handle in self.write_dirty.drain() {
            if let Some(path) = self.write_handles.get(&handle) {
                let asset = cache.get_mut(&handle);

                // write if loaded
                if let Some(asset) = asset {
                    let ty_id = self
                        .handle_to_type
                        .get(&handle)
                        .expect("could not get type id from asset handle");

                    let write_fn = self
                        .write_functions
                        .get(ty_id)
                        .expect("could not get write fn");

                    write_fn(asset, path);
                }
            }
        }
    }

    // checks if any files changed and spawns a thread which reloads the data
    pub fn poll_reload(
        &mut self,
        cache: &mut FxHashMap<DynAssetHandle, DynAsset>,
        render_cache: &mut FxHashMap<RenderAssetKey, DynRenderAsset>,
        render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
        just_loaded: &mut FxHashSet<DynAssetHandle>,
    ) {
        while let Ok(Some(path)) = self.reload_receiver.try_next() {
            println!("1 reload {:?}", path);
            if let Some(handles) = self.reload_handles.get_mut(&path) {
                for handle in handles {
                    println!("reload {:?}", path);
                    just_loaded.insert(*handle);

                    let ty_id = self
                        .handle_to_type
                        .get(handle)
                        .expect("could not get type id from asset handle");

                    // load new fn
                    let loader_fn = self
                        .reload_functions
                        .get(ty_id)
                        .expect("could not get loader fn");
                    let asset = loader_fn(self.load_ctx.clone(), &path);

                    // insert into cache
                    cache.insert(*handle, asset);

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
    render_cache: &mut FxHashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
    handle: DynAssetHandle,
) {
    if let Some(render_types) = render_cache_invalidate_lookup.get(&handle) {
        for render_type in render_types {
            render_cache.remove(&(handle, *render_type));
        }
    }
}

impl<T: Asset + 'static> AssetHandle<T> {
    pub fn loaded(self, cache: &AssetCache) -> bool {
        cache.handle_loaded(self)
    }
    pub fn get(self, cache: &mut AssetCache) -> Option<&T> {
        cache.get(self)
    }
    pub fn get_mut(self, cache: &mut AssetCache) -> Option<&mut T> {
        cache.get_mut(self)
    }
    pub fn convert<G: ConvertableRenderAsset<SourceAsset = T>>(
        self,
        ctx: &mut Context,
        cache: &mut AssetCache,
    ) -> Option<ArcHandle<G>> {
        cache.convert(ctx, self)
    }
}
