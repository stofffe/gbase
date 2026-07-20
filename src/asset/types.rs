use super::{AssetHandle, LoadContext};
use crate::{
    asset::{ConvertContext, LoadAssetResult},
    render::ArcHandle,
    Context,
};
use core::error;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    future::Future,
    path::Path,
};

//
// Types
//

pub type DynAsset = Box<dyn Asset>;
pub type DynAssetHandle = AssetHandle<DynAsset>;
pub type DynAssetLoadFn = Box<dyn Fn() + Send>;
pub type DynAssetLoadFnSync = Box<dyn Fn() -> LoadAssetResult + Send>;

pub type DynDerivedAsset = ArcHandle<dyn Any>;
pub type DerivedAssetKey = (DynAssetHandle, TypeId);

//
// Traits
//

pub trait Asset: Any + Send + Sync {} // TODO: is this even needed? or maybe rename

pub trait AssetSettings: Send + Sync + Clone {}
impl<T: Send + Sync + Clone> AssetSettings for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetError: error::Error + Send {}
impl<T: error::Error + Send> AssetError for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetLoader: Send + Sync {
    type Asset: Asset;
    type Settings: AssetSettings;
    type Error: AssetError;

    #[cfg(not(target_arch = "wasm32"))]
    fn load(
        load_ctx: LoadContext,
        path: &Path,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send;

    #[cfg(target_arch = "wasm32")]
    fn load(
        load_ctx: LoadContext,
        path: &Path,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>>;
}

pub trait DerivedAsset: Any {} // TODO: is this even needed? or maybe rename

pub trait DerivedAssetSettings: Send + Sync {}
impl<T: Send + Sync + Clone> DerivedAssetSettings for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetConverter {
    type SourceAsset: Asset;
    type TargetAsset: DerivedAsset + Clone;
    type Settings: DerivedAssetSettings;
    type Error: error::Error;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset>;
}

pub enum ConvertAssetStatus<T: DerivedAsset> {
    SourceLoading,
    Success(T),
    Failed,
}

pub trait AssetWriter: AssetLoader {
    fn write(asset: &Self::Asset, path: &Path);
}

//
// Other
//

#[derive(thiserror::Error, Debug)]
pub enum EmptyError {}

#[derive(Debug, Clone, Default)]
pub struct NoSettings;
