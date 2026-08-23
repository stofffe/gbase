use crate::{
    asset::{
        Asset, AssetCacheLoad, AssetCacheStorage, AssetHandle, DerivedAsset, DynAssetHandle,
        GetAssetResult, LoadStatus,
    },
    render::ArcHandle,
    Context,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{type_name, Any, TypeId},
    error,
    hash::Hash,
};

//
// Types
//

pub trait DerivedAssetSettings: Hash + Eq + Clone {}
impl<T: Hash + Eq + Clone> DerivedAssetSettings for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetConverter {
    type TargetAsset: DerivedAsset;
    type Settings: DerivedAssetSettings;
    // TODO: is this even being used?
    type Error: error::Error;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset>;
}

pub enum ConvertAssetStatus<T: DerivedAsset> {
    SourceLoading,
    Success(T),
    Failed,
}

//
// Derived
//

pub struct AssetCacheDerived {
    pub(crate) typed_caches: FxHashMap<TypeId, Box<dyn DynDerivedCache>>,
}

impl AssetCacheDerived {
    pub fn new() -> Self {
        Self {
            typed_caches: FxHashMap::default(),
        }
    }

    pub fn get_typed_cache<G: AssetConverter + 'static>(&mut self) -> &mut TypedDerivedCache<G> {
        let entry = self
            .typed_caches
            .entry(TypeId::of::<G>())
            .or_insert(Box::new(TypedDerivedCache::<G>::new()));
        entry
            .as_any()
            .downcast_mut::<TypedDerivedCache<G>>()
            .expect("could not downcast typed derived cache")
    }

    pub fn waiting_dependency_available(&mut self, handle: &DynAssetHandle) {
        tracing::info!("dependency available {}", handle.id());

        for (_, dyn_derived) in self.typed_caches.iter_mut() {
            dyn_derived.dependency_available(handle);
        }
    }

    // TODO: remove pub
    pub fn invalidate_derived_assets_depending_on_handle(&mut self, handle: &DynAssetHandle) {
        tracing::info!("invalidate derived {}", handle.id());
        for (_, dyn_cache) in self.typed_caches.iter_mut() {
            dyn_cache.invalidate(handle);
        }
    }

    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        // check for cached result
        if let Some(render_asset_handle) = self
            .get_typed_cache::<G>()
            .render_cache
            .get(settings)
            .cloned()
        {
            // tracing::info!("get cached conversion");
            return ConvertAssetResult::Success(render_asset_handle);
        }

        // cached status
        if let Some(render_asset_status) = self
            .get_typed_cache::<G>()
            .render_cache_status
            .get(settings)
        {
            // tracing::info!("get cached conversion status");
            return match render_asset_status {
                DerivedAssetStatus::Loading => ConvertAssetResult::Loading,
                DerivedAssetStatus::Failed => ConvertAssetResult::Failed,
            };
        }

        tracing::info!("try conversion for {}", type_name::<G>());
        let mut runtime = ConvertRuntime::new(storage, loader, self);
        let mut convert_ctx = ConvertContext::new(&mut runtime);

        let result = G::convert(ctx, &mut convert_ctx, settings);
        let state = convert_ctx.state.clone();

        let result = match result {
            // Loading
            ConvertAssetStatus::SourceLoading => {
                tracing::error!("conversion loading");

                let typed = self.get_typed_cache::<G>();
                typed.set_status(settings.clone(), DerivedAssetStatus::Loading);
                typed.register_waiting(settings, &state.dependencies);

                ConvertAssetResult::Loading
            }
            // Failed
            ConvertAssetStatus::Failed => {
                tracing::error!("conversion failed");
                let typed = self.get_typed_cache::<G>();

                typed.unregister_waiting(settings);

                match typed.get_last_valid(settings) {
                    Some(asset_handle) => {
                        tracing::warn!(
                            "assert conversion failed, using last valid version instead"
                        );
                        // TODO: maybe clear status here?
                        self.get_typed_cache::<G>()
                            .insert(settings.clone(), asset_handle.clone());
                        ConvertAssetResult::Success(asset_handle.clone())
                    }
                    None => {
                        tracing::error!("asset conversion failed, no last valid version was found");
                        self.get_typed_cache::<G>()
                            .set_status(settings.clone(), DerivedAssetStatus::Loading);
                        ConvertAssetResult::Failed
                    }
                }
            }
            // Success
            ConvertAssetStatus::Success(render_asset_handle) => {
                tracing::warn!("conversion success");
                let render_asset_handle = ArcHandle::new(ctx, render_asset_handle);

                let typed = self.get_typed_cache::<G>();

                typed.unregister_waiting(settings);

                // actual cache
                typed.insert(settings.clone(), render_asset_handle.clone());
                // last valid cache
                typed.insert_last_valid(settings.clone(), render_asset_handle.clone());
                // clear status
                typed.clear_status(settings);

                ConvertAssetResult::Success(render_asset_handle)
            }
        };

        // TODO: should this be called every time? Should it be cleared?
        // self.get_typed_cache::<G>()
        //     .register_dependencies(settings.clone(), &state);

        result
    }
}

//
// Typed/Dyn derive
//

#[derive(Clone)]
pub enum DerivedAssetStatus {
    Loading,
    Failed,
}

pub trait DynDerivedCache {
    fn as_any(&mut self) -> &mut dyn Any;
    fn invalidate(&mut self, handle: &DynAssetHandle);
    fn dependency_available(&mut self, handle: &DynAssetHandle);
}

pub struct TypedDerivedCache<G: AssetConverter> {
    render_cache: FxHashMap<G::Settings, ArcHandle<G::TargetAsset>>,
    render_cache_status: FxHashMap<G::Settings, DerivedAssetStatus>,

    render_cache_last_valid: FxHashMap<G::Settings, ArcHandle<G::TargetAsset>>,

    // all settings which depend on the specified handle
    handle_to_settings: FxHashMap<DynAssetHandle, FxHashSet<G::Settings>>,

    waiting_for: FxHashMap<DynAssetHandle, FxHashSet<G::Settings>>,
}

impl<G: AssetConverter> TypedDerivedCache<G> {
    pub fn new() -> Self {
        Self {
            render_cache: FxHashMap::default(),
            render_cache_last_valid: FxHashMap::default(),
            render_cache_status: FxHashMap::default(),
            handle_to_settings: FxHashMap::default(),
            waiting_for: FxHashMap::default(),
        }
    }

    pub fn insert(
        &mut self,
        settings: G::Settings,
        asset: ArcHandle<G::TargetAsset>,
    ) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache.insert(settings, asset)
    }

    pub fn get(&mut self, settings: &G::Settings) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache.get(settings).cloned()
    }

    pub fn insert_last_valid(
        &mut self,
        settings: G::Settings,
        asset: ArcHandle<G::TargetAsset>,
    ) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache_last_valid.insert(settings, asset)
    }

    pub fn get_last_valid(&mut self, settings: &G::Settings) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache_last_valid.get(settings).cloned()
    }

    pub fn set_status(&mut self, settings: G::Settings, status: DerivedAssetStatus) {
        self.render_cache_status.insert(settings, status);
    }

    pub fn clear_status(&mut self, settings: &G::Settings) {
        self.render_cache_status.remove(settings);
    }

    pub fn get_status(&self, settings: &G::Settings) -> DerivedAssetStatus {
        if let Some(status) = self.render_cache_status.get(settings) {
            status.clone()
        } else {
            DerivedAssetStatus::Failed
        }
    }

    // TODO: unused
    // between derived assets
    pub fn register_dependencies(&mut self, settings: G::Settings, state: &ConvertState) {
        tracing::info!(
            "register {} dependencies for {}",
            state.dependencies.len(),
            type_name::<G>()
        );
        for handle in state.dependencies.iter() {
            self.handle_to_settings
                .entry(handle.clone())
                .or_default()
                .insert(settings.clone());
        }
    }

    pub fn register_waiting(
        &mut self,
        settings: &G::Settings,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        tracing::info!("register waiting: len {}", dependencies.len());
        for dependency in dependencies.iter() {
            self.waiting_for
                .entry(dependency.clone())
                .or_default()
                .insert(settings.clone());
        }
    }

    pub fn unregister_waiting(&mut self, settings: &G::Settings) {
        if self.waiting_for.is_empty() {
            return;
        }

        let len_before = self.waiting_for.len();

        self.waiting_for.retain(|_, dependents| {
            dependents.remove(settings);
            !dependents.is_empty()
        });

        let removed_elems = len_before - self.waiting_for.len();
        if removed_elems > 0 {
            tracing::info!("unregister waiting {}", removed_elems);
        }
    }
}

impl<G: AssetConverter + 'static> DynDerivedCache for TypedDerivedCache<G> {
    fn as_any(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn invalidate(&mut self, handle: &DynAssetHandle) {
        if let Some(settings) = self.handle_to_settings.get(handle) {
            for setting in settings {
                self.render_cache.remove(setting);
                self.render_cache_status.remove(setting);
            }
        }
    }

    fn dependency_available(&mut self, handle: &DynAssetHandle) {
        let Some(waiting_dependents) = self.waiting_for.remove(handle) else {
            return;
        };

        tracing::info!(
            "{}: {}# waiting for handle({})",
            type_name::<G>(),
            waiting_dependents.len(),
            handle.id(),
        );

        // invalidate setting
        for setting in waiting_dependents.iter() {
            self.render_cache_status.remove(setting);
        }
    }
}

//
// Conversion context
//

pub struct ConvertRuntime<'a> {
    pub(crate) storage: &'a mut AssetCacheStorage,
    pub(crate) loader: &'a mut AssetCacheLoad,
    pub(crate) derived: &'a mut AssetCacheDerived,
}

impl<'a> ConvertRuntime<'a> {
    pub fn new(
        storage: &'a mut AssetCacheStorage,
        loader: &'a mut AssetCacheLoad,
        derived: &'a mut AssetCacheDerived,
    ) -> Self {
        Self {
            storage,
            derived,
            loader,
        }
    }
}

#[derive(Clone)]
pub struct ConvertState {
    // track handles used during conversion
    pub dependencies: FxHashSet<DynAssetHandle>,
}

impl ConvertState {
    pub fn new() -> Self {
        Self {
            dependencies: FxHashSet::default(),
        }
    }
}

/// Convertsion context related to a specific conversion
pub struct ConvertContext<'runtime> {
    pub runtime: &'runtime mut ConvertRuntime<'runtime>,
    pub state: ConvertState,
}

impl<'runtime> ConvertContext<'runtime> {
    pub fn new(runtime: &'runtime mut ConvertRuntime<'runtime>) -> Self {
        let state = ConvertState::new();
        Self { runtime, state }
    }

    pub fn get<T: Asset>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        self.state.dependencies.insert(handle.to_dyn());

        if let Some(asset) = self.runtime.storage.get(handle) {
            return GetAssetResult::Success(asset);
        }

        match self.runtime.loader.get_status(&handle.to_dyn()) {
            LoadStatus::Loading => GetAssetResult::Loading,
            LoadStatus::Failed => GetAssetResult::Error,
            LoadStatus::Loaded => {
                panic!("could not get asset from storage but its marked as loaded")
            }
        }
    }

    // TODO: track dependencies when this is called (maybe with depenency enum)
    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.runtime
            .derived
            .convert::<G>(ctx, self.runtime.storage, self.runtime.loader, settings)
    }
}

pub enum ConvertAssetResult<T: DerivedAsset> {
    Loading,
    Success(ArcHandle<T>),
    Failed,
}

impl<T: DerivedAsset> ConvertAssetResult<T> {
    /// Unwrap the result as a success
    ///
    /// Panics for other values than
    pub fn unwrap_success(self) -> ArcHandle<T> {
        match self {
            ConvertAssetResult::Loading => {
                panic!("asset conversion loading: unwrap success failed")
            }
            ConvertAssetResult::Failed => panic!("asset conversion failed: unwrap success failed"),
            ConvertAssetResult::Success(arc_handle) => arc_handle,
        }
    }
}
