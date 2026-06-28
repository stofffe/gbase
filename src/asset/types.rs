use super::{AssetCache, AssetHandle, LoadContext};
use crate::{asset::LoadAssetResult, render::ArcHandle, Context};
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
pub type DynAssetLoadFn = Box<dyn Fn(LoadContext, &Path) -> LoadAssetResult>;

pub type DynDerivedAsset = ArcHandle<dyn Any>;
pub type DerivedAssetKey = (DynAssetHandle, TypeId);

pub type DynLoader = Box<dyn Any>;

//
// Traits
//

pub trait Asset: Any + Send + Sync {} // TODO: is this even needed? or maybe rename

pub trait AssetLoader: Send + Sync + Clone {
    type Asset: Asset;
    type Error: error::Error;

    // TODO: should this be consuming self instead
    fn load(
        &self,
        load_ctx: LoadContext,
        path: &Path,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>>;
}

pub trait AssetWriter: AssetLoader {
    fn write(asset: &Self::Asset, path: &Path);
}

pub trait DerivedAsset: Any {} // TODO: is this even needed? or maybe rename

pub trait AssetConverter {
    type SourceAsset: Asset;
    type TargetAsset: DerivedAsset + Clone;
    type Error: error::Error;

    fn convert(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> ConvertAssetStatus<Self::TargetAsset>;
}

pub enum ConvertAssetStatus<T: DerivedAsset> {
    SourceLoading,
    Success(T),
    Failed,
}

//
// Other
//

#[derive(thiserror::Error, Debug)]
pub enum EmptyError {
    #[error("empty")]
    Err,
}
