use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{type_name, Any, TypeId};

use crate::{
    asset::{
        Asset, AssetCacheStorage, AssetConverter, AssetHandle, ConvertAssetStatus, DerivedAsset,
        DynAssetHandle, GetAssetResult,
    },
    render::ArcHandle,
    Context,
};

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

pub struct AssetCacheDerived {
    pub(crate) typed_caches: FxHashMap<TypeId, Box<dyn DynDerivedCache>>,
}

pub trait DynDerivedCache {
    fn as_any(&mut self) -> &mut dyn Any;
    fn invalidate(&mut self, handle: DynAssetHandle);
}

impl<G: AssetConverter + 'static> DynDerivedCache for TypedDerivedCache<G> {
    fn as_any(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn invalidate(&mut self, handle: DynAssetHandle) {
        if let Some(settings) = self.handle_to_settings.get(&handle) {
            tracing::info!("INVALIDATE TYPED CACHE {}", type_name::<G>());
            for setting in settings {
                self.render_cache.remove(setting);
            }
        }
    }
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

impl AssetCacheDerived {
    pub fn new() -> Self {
        Self {
            typed_caches: FxHashMap::default(),
        }
    }

    // pub fn clear_handle(&mut self, handle: DynAssetHandle) {
    //     if let Some(render_types) = self.render_cache_invalidate_lookup.get(&handle) {
    //         for render_type in render_types {
    //             self.render_cache.remove(&(handle.clone(), *render_type));
    //             self.render_cache_last_valid
    //                 .remove(&(handle.clone(), *render_type));
    //         }
    //     }
    // }

    // pub fn clear_unused_handles(&mut self) {
    //     // TODO: clear all other stuff related to this handle
    //     self.render_cache
    //         .retain(|(handle, _), _| Arc::strong_count(&handle.id) > 1);
    // }

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
        for (_, dyn_cache) in self.typed_caches.iter_mut() {
            dyn_cache.invalidate(handle.clone());
        }
    }

    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        storage: &AssetCacheStorage,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        if let Some(render_asset_handle) = self.get_typed_cache::<G>().get(settings) {
            return ConvertAssetResult::Success(render_asset_handle);
        }

        let mut convert_ctx = ConvertContext::new(storage, self);
        match G::convert(ctx, &mut convert_ctx, settings) {
            ConvertAssetStatus::SourceLoading => ConvertAssetResult::Loading,

            // TODO: insert last valid so we dont hit this each time?
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

pub struct ConvertContext<'a> {
    pub(crate) storage: &'a AssetCacheStorage,
    pub(crate) derived: &'a mut AssetCacheDerived,

    // track handles used during conversion
    dependencies: FxHashSet<DynAssetHandle>,
}

impl<'a> ConvertContext<'a> {
    pub fn new(storage: &'a AssetCacheStorage, derived: &'a mut AssetCacheDerived) -> Self {
        Self {
            storage,
            derived,
            dependencies: FxHashSet::default(),
        }
    }

    pub fn get<T: Asset>(&mut self, handle: AssetHandle<T>) -> GetAssetResult<'a, T> {
        self.dependencies.insert(handle.as_any());
        self.storage.get(handle)
    }

    // TODO: track dependencies when this is called (maybe with depenency enum)
    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.derived.convert::<G>(ctx, self.storage, settings)
    }
}
