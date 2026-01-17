use super::{
    Asset, AssetHandle, AssetLoader, AssetWriter, ConvertableRenderAsset, DynAsset, DynAssetHandle,
    DynAssetLoadFn, DynAssetWriteFn, DynRenderAsset,
};
use crate::{
    asset::{
        self, ConvertAssetStatus, GetAssetResult, GetAssetResultMut, InsertAssetBuilder,
        LoadedAssetBuilder, RenderAssetKey,
    },
    render::ArcHandle,
    Context,
};
use futures_channel::mpsc;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

pub enum LoadAssetResult {
    Loading,
    Success(DynAsset),
    Error,
}

pub enum ConvertAssetResult<T: ConvertableRenderAsset> {
    Loading,
    Success(ArcHandle<T>),
    Failed,
}

impl<T: ConvertableRenderAsset> ConvertAssetResult<T> {
    pub fn unwrap_success(self) -> ArcHandle<T> {
        match self {
            ConvertAssetResult::Loading => panic!("unwrap success failed"),
            ConvertAssetResult::Failed => panic!("unwrap success failed"),
            ConvertAssetResult::Success(arc_handle) => arc_handle,
        }
    }
}

pub struct AssetCache {
    cache: FxHashMap<DynAssetHandle, LoadAssetResult>,

    render_cache: FxHashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_last_valid: FxHashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_invalidate_lookup: FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,

    // async loading
    currently_loading: FxHashSet<DynAssetHandle>,
    just_loaded: FxHashSet<DynAssetHandle>,
    load_sender: mpsc::UnboundedSender<(DynAssetHandle, LoadAssetResult)>,
    load_receiver: mpsc::UnboundedReceiver<(DynAssetHandle, LoadAssetResult)>,

    load_ctx: LoadContext,
    asset_handle_ctx: AssetHandleContext,

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

        let asset_handle_ctx = AssetHandleContext::new();
        let load_ctx = LoadContext::new(load_sender.clone(), asset_handle_ctx.clone());

        Self {
            cache: FxHashMap::default(),

            render_cache: FxHashMap::default(),
            render_cache_last_valid: FxHashMap::default(),
            render_cache_invalidate_lookup: FxHashMap::default(),

            currently_loading: FxHashSet::default(),
            just_loaded: FxHashSet::default(),
            load_sender,
            load_receiver,

            load_ctx,
            asset_handle_ctx,

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
            },
        }
    }

    pub fn load_context(&self) -> &LoadContext {
        &self.load_ctx
    }

    pub fn asset_handle_ctx(&self) -> &AssetHandleContext {
        &self.asset_handle_ctx
    }

    //
    // Assets
    //

    pub fn insert<T: Asset + 'static>(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.cache
            .insert(handle.as_any(), LoadAssetResult::Success(Box::new(data)));
        handle
    }

    pub fn get<'a, T: Asset + 'static>(&'a self, handle: AssetHandle<T>) -> GetAssetResult<'a, T> {
        let Some(asset) = self.cache.get(&handle.as_any()) else {
            return GetAssetResult::Failed;
        };

        let LoadAssetResult::Success(asset) = asset else {
            return GetAssetResult::Loading;
        };

        // TODO: errors as well?
        let asset = (asset.as_ref() as &dyn Any)
            .downcast_ref::<T>()
            .expect("could not downcast");

        GetAssetResult::Loaded(asset)
    }

    pub fn get_mut<'a, T: Asset + 'static>(
        &'a mut self,
        handle: AssetHandle<T>,
    ) -> GetAssetResultMut<'a, T> {
        let Some(asset) = self.cache.get_mut(&handle.as_any()) else {
            return GetAssetResultMut::Failed;
        };

        let LoadAssetResult::Success(asset) = asset else {
            return GetAssetResultMut::Loading;
        };

        let asset = (asset.as_mut() as &mut dyn Any)
            .downcast_mut::<T>()
            .expect("could not downcast");

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

        GetAssetResultMut::Loaded(asset)
    }

    //
    // Asset builders
    //

    pub fn insert_builder<T: Asset>(&mut self, value: T) -> InsertAssetBuilder<T> {
        asset::AssetBuilder::insert(value)
    }

    pub fn load_builder<T: AssetLoader + 'static>(
        &mut self,
        path: impl Into<PathBuf>,
        loader: T,
    ) -> LoadedAssetBuilder<T> {
        asset::AssetBuilder::load(self, path, loader)
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
        let handle_clone = handle.clone();
        let loaded_sender_clone = self.load_sender.clone();
        let load_context = self.load_ctx.clone();

        // TODO: insert loading before actually loading

        // load async
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            pollster::block_on(async {
                let data = loader.load(load_context, &path_clone).await;

                match data {
                    Ok(asset) => loaded_sender_clone
                        .unbounded_send((
                            handle_clone.as_any(),
                            LoadAssetResult::Success(Box::new(asset)),
                        ))
                        .expect("could not send"),
                    Err(err) => {
                        tracing::error!("error loading asset {:?}: {}", path, err);
                        loaded_sender_clone
                            .unbounded_send((handle_clone.as_any(), LoadAssetResult::Error))
                            .expect("could not send");
                    }
                }
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
    ) -> ConvertAssetResult<G> {
        let key = (handle.clone().as_any(), TypeId::of::<G>());

        let render_asset_handle = match self.render_cache.get(&key) {
            Some(render_asset_handle) => render_asset_handle.clone(),
            // Some(render_asset_handle) => todo!(),
            None => {
                match G::convert(ctx, self, handle.clone()) {
                    ConvertAssetStatus::Loading => return ConvertAssetResult::Loading,

                    // TODO: insert last valid so we dont hit this each time?
                    ConvertAssetStatus::Failed => match self.render_cache_last_valid.get(&key) {
                        Some(asset_handle) => {
                            tracing::warn!(
                                "assert conversion failed, using last valid version instead"
                            );
                            self.render_cache.insert(key.clone(), asset_handle.clone());
                            asset_handle.clone()
                        }
                        None => {
                            tracing::error!(
                                "asset conversion failed, no last valid version was found"
                            );
                            return ConvertAssetResult::Failed;
                        }
                    },

                    ConvertAssetStatus::Success(render_asset_handle) => {
                        let render_asset_any_handle =
                            ArcHandle::new(ctx, render_asset_handle).upcast();
                        // actual cache
                        self.render_cache
                            .insert(key.clone(), render_asset_any_handle.clone());
                        // last valid cache
                        self.render_cache_last_valid
                            .insert(key.clone(), render_asset_any_handle.clone());
                        // invalidate lookup
                        self.render_cache_invalidate_lookup
                            .entry(handle.as_any())
                            .or_default()
                            .insert(TypeId::of::<G>());

                        render_asset_any_handle
                    }
                }
            }
        };

        let typed_handle = render_asset_handle
            .downcast::<G>()
            .expect("could not downcast render any handle");

        ConvertAssetResult::Success(typed_handle)
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
                self.load_ctx.clone(),
            );
            self.ext.poll_write(&mut self.cache);
        }

        self.poll_loaded();
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(&mut self) {
        while let Ok(Some((handle, asset))) = self.load_receiver.try_next() {
            // insert in cache
            self.cache.insert(handle.clone(), asset);

            // remove from currently loaded
            self.currently_loading.remove(&handle.as_any());

            // invalidate render cache
            invalidate_render_cache(
                &mut self.render_cache,
                &self.render_cache_invalidate_lookup,
                handle.clone(),
            );

            self.just_loaded.insert(handle);
        }
    }

    pub fn clear_cpu_handles(&mut self) {
        // TODO: clear all other stuff related to this handle
        self.cache
            .retain(|handle, _| Arc::strong_count(&handle.id) > 1);
    }

    pub fn clear_gpu_handles(&mut self) {
        // TODO: clear all other stuff related to this handle
        self.render_cache
            .retain(|(handle, _), _| Arc::strong_count(&handle.id) > 1);
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
    sender: mpsc::UnboundedSender<(DynAssetHandle, LoadAssetResult)>,
    asset_handle_ctx: AssetHandleContext,
}

impl LoadContext {
    pub fn new(
        sender: mpsc::UnboundedSender<(DynAssetHandle, LoadAssetResult)>,
        asset_handle_ctx: AssetHandleContext,
    ) -> Self {
        Self {
            sender,
            asset_handle_ctx,
        }
    }

    pub fn insert<T: Asset>(&self, value: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.sender
            .unbounded_send((handle.as_any(), LoadAssetResult::Success(Box::new(value))))
            .expect("could not send asset handle");
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
    reload_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
    reload_receiver: mpsc::UnboundedReceiver<PathBuf>,

    // writing
    write_handles: FxHashMap<DynAssetHandle, PathBuf>,
    write_functions: FxHashMap<TypeId, DynAssetWriteFn>,
    write_dirty: FxHashSet<DynAssetHandle>,
}

#[derive(Debug, Clone)]
pub struct AssetHandleContext {
    id: Arc<Mutex<u64>>,
}

impl AssetHandleContext {
    fn new() -> Self {
        Self {
            id: Arc::new(Mutex::new(0)),
        }
    }
    pub fn next_id(&self) -> u64 {
        let mut id_guard = self.id.lock().expect("could not unlock asset id lock");
        let id = *id_guard;
        *id_guard += 1;
        id
    }
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
        let absolute_path = fs::canonicalize(path)
            .unwrap_or_else(|err| panic!("could not find path {}: {:?}", path.display(), err));

        // start watching path
        self.reload_watcher
            .watcher()
            .watch(
                &absolute_path,
                notify_debouncer_mini::notify::RecursiveMode::Recursive, // TODO: non recursive?
            )
            .unwrap_or_else(|err| panic!("could not watch {}: {:?}", absolute_path.display(), err));

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
                    let result = pollster::block_on(loader.clone().load(load_ctx, path));
                    match result {
                        Ok(asset) => LoadAssetResult::Success(Box::new(asset)),
                        Err(err) => {
                            tracing::error!("could not reload asset {:?}: {}", path, err);
                            LoadAssetResult::Error
                        }
                    }
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

        // TODO:
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
    pub fn poll_write(&mut self, cache: &mut FxHashMap<DynAssetHandle, LoadAssetResult>) {
        for handle in self.write_dirty.drain() {
            if let Some(path) = self.write_handles.get(&handle) {
                let asset = cache.get_mut(&handle);

                // write if loaded
                if let Some(asset) = asset {
                    let LoadAssetResult::Success(asset) = asset else {
                        panic!("tried to poll write on asset not loaded");
                    };

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
        cache: &mut FxHashMap<DynAssetHandle, LoadAssetResult>,
        render_cache: &mut FxHashMap<RenderAssetKey, DynRenderAsset>,
        render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
        just_loaded: &mut FxHashSet<DynAssetHandle>,
        load_ctx: LoadContext,
    ) {
        while let Ok(Some(path)) = self.reload_receiver.try_next() {
            if let Some(handles) = self.reload_handles.get_mut(&path) {
                for handle in handles {
                    // println!("reload {:?}", path);
                    just_loaded.insert(handle.clone());

                    let ty_id = self
                        .handle_to_type
                        .get(handle)
                        .expect("could not get type id from asset handle");

                    // load new fn
                    let loader_fn = self
                        .reload_functions
                        .get(ty_id)
                        .expect("could not get loader fn");
                    let asset = loader_fn(load_ctx.clone(), &path);

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
    render_cache: &mut FxHashMap<RenderAssetKey, DynRenderAsset>,
    render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
    handle: DynAssetHandle,
) {
    if let Some(render_types) = render_cache_invalidate_lookup.get(&handle) {
        for render_type in render_types {
            render_cache.remove(&(handle.clone(), *render_type));
        }
    }
}

impl<T: Asset + 'static> AssetHandle<T> {
    pub fn loaded(&self, cache: &AssetCache) -> bool {
        cache.handle_loaded(self.clone())
    }
    pub fn just_loaded(&self, cache: &AssetCache) -> bool {
        cache.handle_just_loaded(self.clone())
    }
    pub fn get<'a>(&self, cache: &'a mut AssetCache) -> GetAssetResult<'a, T> {
        cache.get(self.clone())
    }
    pub fn get_mut<'a>(&self, cache: &'a mut AssetCache) -> GetAssetResultMut<'a, T> {
        cache.get_mut(self.clone())
    }
    pub fn convert<G: ConvertableRenderAsset<SourceAsset = T>>(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
    ) -> ConvertAssetResult<G> {
        cache.convert(ctx, self.clone())
    }
}
