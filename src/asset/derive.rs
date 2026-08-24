use rustc_hash::FxHashSet;
use winit::keyboard::NamedKey::New;

use crate::{
    asset::{
        Asset, AssetCacheDerivedRegistry, AssetCacheDerivedStorage, AssetCacheLoad,
        AssetCacheStorage, AssetHandle, ConvertRequest, DerivedAsset, DynAssetHandle,
        DynConvertRequest, DynDerivedHandle, GetAssetResult, LoadStatus,
    },
    render::ArcHandle,
    Context,
};
use std::{error, fmt::Display, hash::Hash};

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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum DynDependency {
    Asset(DynAssetHandle),
    Derived(DynDerivedHandle),
}

impl Display for DynDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynDependency::Asset(dyn_asset_handle) => {
                write!(f, "[{}: dyn dependency]", dyn_asset_handle)
            }
            DynDependency::Derived(dyn_derived_handle) => {
                write!(f, "[{}: dyn dependency]", dyn_derived_handle)
            }
        }
    }
}

pub enum ConvertAssetStatus<T: DerivedAsset> {
    Loading,
    Success(T),
    Failed,
}

//
// Conversion context
//

pub struct ConvertRuntime<'a> {
    // to get assets
    pub(crate) storage: &'a mut AssetCacheStorage,
    // to get asset status
    pub(crate) loader: &'a mut AssetCacheLoad,
    // to get derived assets
    pub(crate) derived_storage: &'a mut AssetCacheDerivedStorage,
    // to get setting -> derived handle mapping
    pub(crate) derived_registry: &'a mut AssetCacheDerivedRegistry,
}

impl<'a> ConvertRuntime<'a> {
    pub fn new(
        storage: &'a mut AssetCacheStorage,
        loader: &'a mut AssetCacheLoad,
        derived_storage: &'a mut AssetCacheDerivedStorage,
        derived_registry: &'a mut AssetCacheDerivedRegistry,
    ) -> Self {
        Self {
            storage,
            loader,
            derived_storage,
            derived_registry,
        }
    }
}

pub struct ConvertState {
    // The dependency that caused the conversion to return waiting
    pub wait_for: Option<DynDependency>,

    // If wait_for was a nested conversion that resulted in a new handle, store the request here
    pub conversion_request: Option<Box<dyn DynConvertRequest>>,

    // All dependencies accessed during the conversion
    pub dependencies: FxHashSet<DynDependency>,
}

impl ConvertState {
    pub fn new() -> Self {
        Self {
            wait_for: None,
            conversion_request: None,
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

    pub fn get_load_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        // register deps
        self.state.dependencies.insert(handle.to_dyn_dependency());

        if let Some(asset) = self.runtime.storage.get(handle) {
            return GetAssetResult::Success(asset);
        }

        match self.runtime.loader.get_status(&handle.to_dyn()) {
            LoadStatus::Loading => {
                self.state.wait_for = Some(DynDependency::Asset(handle.to_dyn()));
                tracing::info!("{} is not ready, set blocking", handle);
                GetAssetResult::Loading
            }
            LoadStatus::Failed => GetAssetResult::Error,
            LoadStatus::Loaded => {
                panic!("could not get asset from storage but its marked as loaded")
            }
        }
    }

    // TODO: track dependencies when this is called (maybe with depenency enum)
    // rename to convert later
    pub fn get_nested_convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        // get handle from registry
        let (handle, created_new) = self
            .runtime
            .derived_registry
            .get_or_create_handle::<G>(settings.clone());

        // register deps
        self.state.dependencies.insert(handle.to_dyn_dependency());

        self.state.wait_for = Some(DynDependency::Derived(handle.to_dyn()));
        if created_new {
            self.state.conversion_request = Some(Box::new(ConvertRequest::<G>::new(
                handle.clone(),
                settings.clone(),
            )));
        }

        match self.runtime.derived_storage.get(&handle) {
            Some(asset) => ConvertAssetResult::Success(asset),
            None => {
                self.state.wait_for = Some(DynDependency::Derived(handle.to_dyn()));
                ConvertAssetResult::Loading
            }
        }
    }
}

// NOTE: user facing
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

//
// Derived
//

// pub struct AssetCacheDerived {
//     pub(crate) typed_caches: FxHashMap<TypeId, Box<dyn DynDerivedCache>>,
// }
//
// impl AssetCacheDerived {
//     pub fn new() -> Self {
//         Self {
//             typed_caches: FxHashMap::default(),
//         }
//     }
//
//     pub fn get_typed_cache<G: AssetConverter + 'static>(&mut self) -> &mut TypedDerivedCache<G> {
//         let entry = self
//             .typed_caches
//             .entry(TypeId::of::<G>())
//             .or_insert(Box::new(TypedDerivedCache::<G>::new()));
//         entry
//             .as_any()
//             .downcast_mut::<TypedDerivedCache<G>>()
//             .expect("could not downcast typed derived cache")
//     }
//
//     pub fn waiting_dependency_available(&mut self, handle: &DynAssetHandle) {
//         tracing::info!("dependency available {}", handle);
//
//         for (_, dyn_derived) in self.typed_caches.iter_mut() {
//             dyn_derived.dependency_available(handle);
//         }
//     }
//
//     // TODO: remove pub
//     pub fn invalidate_derived_assets_depending_on_handle(&mut self, handle: &DynAssetHandle) {
//         tracing::info!("invalidate derived {}", handle);
//         for (_, dyn_cache) in self.typed_caches.iter_mut() {
//             dyn_cache.invalidate(handle);
//         }
//     }
//
//     pub fn convert<G: AssetConverter + 'static>(
//         &mut self,
//         ctx: &mut Context,
//         storage: &mut AssetCacheStorage,
//         loader: &mut AssetCacheLoad,
//         derive_storage: &mut AssetCacheDerivedStorage,
//         derive_convert: &mut AssetCacheDerivedConvert,
//         derive_registry: &mut AssetCacheDerivedRegistry,
//         settings: &G::Settings,
//     ) -> ConvertAssetResult<G::TargetAsset> {
//         todo!()
//         // // check for cached result
//         // if let Some(render_asset_handle) = self
//         //     .get_typed_cache::<G>()
//         //     .render_cache
//         //     .get(settings)
//         //     .cloned()
//         // {
//         //     // tracing::info!("get cached conversion");
//         //     return ConvertAssetResult::Success(render_asset_handle);
//         // }
//         //
//         // // cached status
//         // if let Some(render_asset_status) = self
//         //     .get_typed_cache::<G>()
//         //     .render_cache_status
//         //     .get(settings)
//         // {
//         //     // tracing::info!("get cached conversion status");
//         //     return match render_asset_status {
//         //         DerivedAssetStatus::Loading => ConvertAssetResult::Loading,
//         //         DerivedAssetStatus::Failed => ConvertAssetResult::Failed,
//         //     };
//         // }
//         //
//         // tracing::info!("try conversion for {}", type_name::<G>());
//         // let mut runtime = ConvertRuntime::new(storage, loader, derive_storage, derive_registry);
//         // let mut convert_ctx = ConvertContext::new(&mut runtime);
//         //
//         // let result = G::convert(ctx, &mut convert_ctx, settings);
//         // let state = convert_ctx.state.clone();
//         //
//         // let result = match result {
//         //     // Loading
//         //     ConvertAssetStatus::Loading => {
//         //         tracing::error!("conversion loading");
//         //
//         //         let typed = self.get_typed_cache::<G>();
//         //         typed.set_status(settings.clone(), DerivedAssetStatus::Loading);
//         //         // typed.register_waiting(settings, &state.blocking_dependencies);
//         //
//         //         ConvertAssetResult::Loading
//         //     }
//         //     // Failed
//         //     ConvertAssetStatus::Failed => {
//         //         tracing::error!("conversion failed");
//         //         let typed = self.get_typed_cache::<G>();
//         //
//         //         typed.unregister_waiting(settings);
//         //
//         //         match typed.get_last_valid(settings) {
//         //             Some(asset_handle) => {
//         //                 tracing::warn!(
//         //                     "assert conversion failed, using last valid version instead"
//         //                 );
//         //                 // TODO: maybe clear status here?
//         //                 self.get_typed_cache::<G>()
//         //                     .insert(settings.clone(), asset_handle.clone());
//         //                 ConvertAssetResult::Success(asset_handle.clone())
//         //             }
//         //             None => {
//         //                 tracing::error!("asset conversion failed, no last valid version was found");
//         //                 self.get_typed_cache::<G>()
//         //                     .set_status(settings.clone(), DerivedAssetStatus::Loading);
//         //                 ConvertAssetResult::Failed
//         //             }
//         //         }
//         //     }
//         //     // Success
//         //     ConvertAssetStatus::Success(render_asset_handle) => {
//         //         tracing::warn!("conversion success");
//         //         let render_asset_handle = ArcHandle::new(ctx, render_asset_handle);
//         //
//         //         let typed = self.get_typed_cache::<G>();
//         //
//         //         typed.unregister_waiting(settings);
//         //
//         //         // actual cache
//         //         typed.insert(settings.clone(), render_asset_handle.clone());
//         //         // last valid cache
//         //         typed.insert_last_valid(settings.clone(), render_asset_handle.clone());
//         //         // clear status
//         //         typed.clear_status(settings);
//         //
//         //         ConvertAssetResult::Success(render_asset_handle)
//         //     }
//         // };
//         //
//         // // TODO: should this be called every time? Should it be cleared?
//         // // self.get_typed_cache::<G>()
//         // //     .register_dependencies(settings.clone(), &state);
//         //
//         // result
//     }
// }
//
// //
// // Typed/Dyn derive
// //
//
// #[derive(Clone)]
// pub enum DerivedAssetStatus {
//     Loading,
//     Failed,
// }
//
// pub trait DynDerivedCache {
//     fn as_any(&mut self) -> &mut dyn Any;
//     fn invalidate(&mut self, handle: &DynAssetHandle);
//     fn dependency_available(&mut self, handle: &DynAssetHandle);
// }
//
// pub struct TypedDerivedCache<G: AssetConverter> {
//     render_cache: FxHashMap<G::Settings, ArcHandle<G::TargetAsset>>,
//     render_cache_status: FxHashMap<G::Settings, DerivedAssetStatus>,
//
//     render_cache_last_valid: FxHashMap<G::Settings, ArcHandle<G::TargetAsset>>,
//
//     // all settings which depend on the specified handle
//     handle_to_settings: FxHashMap<DynAssetHandle, FxHashSet<G::Settings>>,
//
//     waiting_for: FxHashMap<DynAssetHandle, FxHashSet<G::Settings>>,
// }

// impl<G: AssetConverter> TypedDerivedCache<G> {
//     pub fn new() -> Self {
//         Self {
//             render_cache: FxHashMap::default(),
//             render_cache_last_valid: FxHashMap::default(),
//             render_cache_status: FxHashMap::default(),
//             handle_to_settings: FxHashMap::default(),
//             waiting_for: FxHashMap::default(),
//         }
//     }
//
//     pub fn insert(
//         &mut self,
//         settings: G::Settings,
//         asset: ArcHandle<G::TargetAsset>,
//     ) -> Option<ArcHandle<G::TargetAsset>> {
//         self.render_cache.insert(settings, asset)
//     }
//
//     pub fn get(&mut self, settings: &G::Settings) -> Option<ArcHandle<G::TargetAsset>> {
//         self.render_cache.get(settings).cloned()
//     }
//
//     pub fn insert_last_valid(
//         &mut self,
//         settings: G::Settings,
//         asset: ArcHandle<G::TargetAsset>,
//     ) -> Option<ArcHandle<G::TargetAsset>> {
//         self.render_cache_last_valid.insert(settings, asset)
//     }
//
//     pub fn get_last_valid(&mut self, settings: &G::Settings) -> Option<ArcHandle<G::TargetAsset>> {
//         self.render_cache_last_valid.get(settings).cloned()
//     }
//
//     pub fn set_status(&mut self, settings: G::Settings, status: DerivedAssetStatus) {
//         self.render_cache_status.insert(settings, status);
//     }
//
//     pub fn clear_status(&mut self, settings: &G::Settings) {
//         self.render_cache_status.remove(settings);
//     }
//
//     pub fn get_status(&self, settings: &G::Settings) -> DerivedAssetStatus {
//         if let Some(status) = self.render_cache_status.get(settings) {
//             status.clone()
//         } else {
//             DerivedAssetStatus::Failed
//         }
//     }
//
//     // // between derived assets
//     // pub fn register_dependencies(&mut self, settings: G::Settings, state: &ConvertState) {
//     //     tracing::info!(
//     //         "register {} dependencies for {}",
//     //         state.dependencies.len(),
//     //         type_name::<G>()
//     //     );
//     //     for handle in state.dependencies.iter() {
//     //         self.handle_to_settings
//     //             .entry(handle.clone())
//     //             .or_default()
//     //             .insert(settings.clone());
//     //     }
//     // }
//     //
//     // pub fn register_waiting(
//     //     &mut self,
//     //     settings: &G::Settings,
//     //     dependencies: &FxHashSet<DynAssetHandle>,
//     // ) {
//     //     for dependency in dependencies.iter() {
//     //         tracing::info!("register waiting on {}", dependency);
//     //         self.waiting_for
//     //             .entry(dependency.clone())
//     //             .or_default()
//     //             .insert(settings.clone());
//     //     }
//     // }
//
//     // pub fn unregister_waiting(&mut self, settings: &G::Settings) {
//     //     if self.waiting_for.is_empty() {
//     //         return;
//     //     }
//     //
//     //     let len_before = self.waiting_for.len();
//     //
//     //     self.waiting_for.retain(|_, dependents| {
//     //         dependents.remove(settings);
//     //         !dependents.is_empty()
//     //     });
//     //
//     //     let removed_elems = len_before - self.waiting_for.len();
//     //     if removed_elems > 0 {
//     //         tracing::info!("unregister waiting {}", removed_elems);
//     //     }
//     // }
// }

// impl<G: AssetConverter + 'static> DynDerivedCache for TypedDerivedCache<G> {
//     fn as_any(&mut self) -> &mut dyn Any {
//         self as &mut dyn Any
//     }
//
//     fn invalidate(&mut self, handle: &DynAssetHandle) {
//         if let Some(settings) = self.handle_to_settings.get(handle) {
//             for setting in settings {
//                 self.render_cache.remove(setting);
//                 self.render_cache_status.remove(setting);
//             }
//         }
//     }
//
//     fn dependency_available(&mut self, handle: &DynAssetHandle) {
//         let Some(waiting_dependents) = self.waiting_for.remove(handle) else {
//             return;
//         };
//
//         tracing::info!(
//             "{}: {}# waiting for handle({})",
//             type_name::<G>(),
//             waiting_dependents.len(),
//             handle,
//         );
//
//         // invalidate setting
//         for setting in waiting_dependents.iter() {
//             self.render_cache_status.remove(setting);
//         }
//     }
// }
