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
pub type DynRenderAsset = ArcHandle<dyn Any>;
pub type DynAssetLoadFn = Box<dyn Fn(LoadContext, &Path) -> LoadAssetResult>;
pub type DynAssetWriteFn = Box<dyn Fn(&mut DynAsset, &Path)>;
pub type DynAssetOnLoadFn = Box<dyn Fn(&mut DynAsset)>;
pub type TypedAssetOnLoadFn<T> = Box<dyn Fn(&mut T)>;
pub type RenderAssetKey = (DynAssetHandle, TypeId);

//
// Traits
//

pub trait Asset: Any + Send + Sync {} // TODO: is this even needed? or maybe rename

pub trait AssetLoader: Send + Sync + Clone {
    type Asset: Asset;
    type Error: error::Error;

    fn load(
        &self,
        load_ctx: LoadContext,
        path: &Path,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>>;
}

pub trait AssetWriter: AssetLoader {
    fn write(asset: &Self::Asset, path: &Path);
}

pub trait RenderAsset: Any {} // TODO: is this even needed? or maybe rename

// TODO: should this actually return arc handles or should caching system handle that?
pub trait ConvertableRenderAsset: RenderAsset + Clone {
    type SourceAsset: Asset;
    type Error: error::Error;

    fn convert(
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> ConvertAssetStatus<Self>;
}

//
// Other
//

// TODO: should this be archandle or just arc?
pub enum ConvertAssetStatus<T: ConvertableRenderAsset> {
    Loading,
    Success(T),
    Failed,
}

#[derive(thiserror::Error, Debug)]
pub enum EmptyError {
    #[error("empty")]
    Err,
}
