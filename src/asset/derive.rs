use crate::{
    asset::{
        Asset, AssetCacheLoad, AssetCacheStorage, AssetHandle, DynAssetHandle, GetAssetResult,
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

pub trait DerivedAsset: Any + Send + Sync {} // TODO: is this even needed? or maybe rename

pub trait DerivedAssetSettings: Send + Hash + Eq + Clone {}
impl<T: Send + Hash + Eq + Clone> DerivedAssetSettings for T {} // TODO: maybe do this for Asset and derived asset
                                                                //
pub type DynDerivedAsset = ArcHandle<dyn Any + Send + Sync>;

pub trait AssetConverter {
    type TargetAsset: DerivedAsset;
    type Settings: DerivedAssetSettings;
    // TODO: is this even being used?
    type Error: error::Error;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_, '_>, // TODO: should this be mutable reference?
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

    pub fn invalidate_derived_assets_depending_on_handle(&mut self, handle: DynAssetHandle) {
        tracing::error!("INVALIDATE DERIVED {}", handle.id());
        for (_, dyn_cache) in self.typed_caches.iter_mut() {
            dyn_cache.invalidate(handle.clone());
        }
    }

    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        if let Some(render_asset_handle) = self.get_typed_cache::<G>().get(settings) {
            return ConvertAssetResult::Success(render_asset_handle);
        }

        let mut convert_ctx = ConvertContext::new(storage, loader, self);
        match G::convert(ctx, &mut convert_ctx, settings) {
            ConvertAssetStatus::SourceLoading => ConvertAssetResult::Loading,
            ConvertAssetStatus::Failed => {
                match self.get_typed_cache::<G>().get_last_valid(settings) {
                    Some(asset_handle) => {
                        tracing::warn!(
                            "assert conversion failed, using last valid version instead"
                        );
                        self.get_typed_cache::<G>()
                            .insert(settings.clone(), asset_handle.clone());
                        ConvertAssetResult::Success(asset_handle.clone())
                    }
                    None => {
                        tracing::error!("asset conversion failed, no last valid version was found");
                        ConvertAssetResult::Failed
                    }
                }
            }
            ConvertAssetStatus::Success(render_asset_handle) => {
                // tracing::info!("CONVERSION SUCCESS {:?}", render_asset_handle.type_id());
                let render_asset_handle = ArcHandle::new(ctx, render_asset_handle);

                let deps = convert_ctx.dependencies.clone();

                let typed_cache = self.get_typed_cache::<G>();
                // actual cache
                typed_cache.insert(settings.clone(), render_asset_handle.clone());
                // last valid cache
                typed_cache.insert_last_valid(settings.clone(), render_asset_handle.clone());
                // register dependencies
                typed_cache.register_dependencies(settings.clone(), &deps);

                ConvertAssetResult::Success(render_asset_handle)
            }
        }
    }
}

//
// Typed/Dyn derive
//

pub trait DynDerivedCache {
    fn as_any(&mut self) -> &mut dyn Any;
    fn invalidate(&mut self, handle: DynAssetHandle);
}

pub struct TypedDerivedCache<G: AssetConverter> {
    pub(crate) render_cache: FxHashMap<G::Settings, ArcHandle<G::TargetAsset>>,
    pub(crate) render_cache_last_valid: FxHashMap<G::Settings, ArcHandle<G::TargetAsset>>,
    pub(crate) handle_to_settings: FxHashMap<DynAssetHandle, FxHashSet<G::Settings>>,
}

impl<G: AssetConverter> TypedDerivedCache<G> {
    pub fn new() -> Self {
        Self {
            render_cache: FxHashMap::default(),
            render_cache_last_valid: FxHashMap::default(),
            handle_to_settings: FxHashMap::default(),
        }
    }

    pub fn get(&mut self, settings: &G::Settings) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache.get(settings).cloned()
    }

    pub fn get_last_valid(&mut self, settings: &G::Settings) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache_last_valid.get(settings).cloned()
    }

    pub fn insert(
        &mut self,
        settings: G::Settings,
        asset: ArcHandle<G::TargetAsset>,
    ) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache.insert(settings, asset)
    }

    pub fn insert_last_valid(
        &mut self,
        settings: G::Settings,
        asset: ArcHandle<G::TargetAsset>,
    ) -> Option<ArcHandle<G::TargetAsset>> {
        self.render_cache_last_valid.insert(settings, asset)
    }

    pub fn register_dependencies(
        &mut self,
        settings: G::Settings,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        for handle in dependencies.iter() {
            self.handle_to_settings
                .entry(handle.clone())
                .or_default()
                .insert(settings.clone());
        }
    }
}

impl<G: AssetConverter + 'static> DynDerivedCache for TypedDerivedCache<G> {
    fn as_any(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn invalidate(&mut self, handle: DynAssetHandle) {
        if let Some(settings) = self.handle_to_settings.get(&handle) {
            for setting in settings {
                self.render_cache.remove(setting);
            }
        }
    }
}

//
// Conversion context
//

/// Convertsion context related to a specific conversion
pub struct ConvertContext<'storage, 'derived> {
    pub(crate) storage: &'storage mut AssetCacheStorage,
    pub(crate) loader: &'storage mut AssetCacheLoad,
    pub(crate) derived: &'derived mut AssetCacheDerived,

    // track handles used during conversion
    dependencies: FxHashSet<DynAssetHandle>,
}

impl<'storage, 'derived> ConvertContext<'storage, 'derived> {
    pub fn new(
        storage: &'storage mut AssetCacheStorage,
        loader: &'storage mut AssetCacheLoad,
        derived: &'derived mut AssetCacheDerived,
    ) -> Self {
        Self {
            storage,
            derived,
            loader,
            dependencies: FxHashSet::default(),
        }
    }

    pub fn get<T: Asset>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        self.dependencies.insert(handle.as_any());

        if let Some(asset) = self.storage.get(handle) {
            return GetAssetResult::Success(asset);
        }

        match self.loader.get_status::<T>(handle) {
            super::LoadStatus::Loading => GetAssetResult::Loading,
            super::LoadStatus::Failed => GetAssetResult::Error,
        }
    }

    // TODO: track dependencies when this is called (maybe with depenency enum)
    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.derived
            .convert::<G>(ctx, self.storage, self.loader, settings)
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
