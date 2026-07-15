use std::{any::TypeId, sync::Arc};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    asset::{
        Asset, AssetCacheStorage, AssetConverter, AssetHandle, ConvertAssetStatus, DerivedAsset,
        DerivedAssetKey, DynAssetHandle, DynDerivedAsset, GetAssetResult,
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
    // derived cache
    // TODO: create new AssetCacheDerived struct
    pub(crate) render_cache: FxHashMap<DerivedAssetKey, DynDerivedAsset>,
    pub(crate) render_cache_last_valid: FxHashMap<DerivedAssetKey, DynDerivedAsset>,
    pub(crate) render_cache_invalidate_lookup: FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
}

impl AssetCacheDerived {
    pub fn new() -> Self {
        Self {
            render_cache: FxHashMap::default(),
            render_cache_last_valid: FxHashMap::default(),
            render_cache_invalidate_lookup: FxHashMap::default(),
        }
    }

    pub fn clear_unused_handles(&mut self) {
        // TODO: clear all other stuff related to this handle
        self.render_cache
            .retain(|(handle, _), _| Arc::strong_count(&handle.id) > 1);
    }

    pub fn invalidate_render_cache_for_handle(&mut self, handle: DynAssetHandle) {
        if let Some(render_types) = self.render_cache_invalidate_lookup.get(&handle) {
            for render_type in render_types {
                self.render_cache.remove(&(handle.clone(), *render_type));
            }
        }
    }

    pub fn convert<G: AssetConverter>(
        &mut self,
        ctx: &mut Context,
        storage: &AssetCacheStorage,
        handle: AssetHandle<G::SourceAsset>,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        let key = (handle.clone().as_any(), TypeId::of::<G::TargetAsset>());

        let render_asset_handle = match self.render_cache.get(&key) {
            Some(render_asset_handle) => render_asset_handle.clone(),
            None => {
                match G::convert(
                    ctx,
                    &mut ConvertContext::new(storage, self),
                    handle.clone(),
                    settings,
                ) {
                    ConvertAssetStatus::SourceLoading => return ConvertAssetResult::Loading,

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
                            .insert(TypeId::of::<G::TargetAsset>());

                        render_asset_any_handle
                    }
                }
            }
        };

        let typed_handle = render_asset_handle
            .downcast::<G::TargetAsset>()
            .expect("could not downcast render any handle");

        ConvertAssetResult::Success(typed_handle)
    }
}

pub struct ConvertContext<'a> {
    pub(crate) storage: &'a AssetCacheStorage,
    pub(crate) converter: &'a mut AssetCacheDerived,
}

impl<'a> ConvertContext<'a> {
    pub fn new(storage: &'a AssetCacheStorage, converter: &'a mut AssetCacheDerived) -> Self {
        Self { storage, converter }
    }

    pub fn get<T: Asset>(&'a self, handle: AssetHandle<T>) -> GetAssetResult<'a, T> {
        self.storage.get(handle)
    }

    pub fn convert_custom_settings<G: AssetConverter>(
        &mut self,
        ctx: &mut Context,
        handle: AssetHandle<G::SourceAsset>,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.converter
            .convert::<G>(ctx, self.storage, handle, settings)
    }

    pub fn convert_default_settings<G: AssetConverter<Settings: Default>>(
        &mut self,
        ctx: &mut Context,
        handle: AssetHandle<G::SourceAsset>,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.converter
            .convert::<G>(ctx, self.storage, handle, &G::Settings::default())
    }
}
