use super::{AssetCache, AssetHandle, LoadContext};
use crate::{render::ArcHandle, Context};
use std::{
    any::Any,
    fmt::{Debug, Display},
    future::Future,
    path::Path,
};

pub type DynAsset = Box<dyn Asset>;
pub type DynAssetHandle = AssetHandle<DynAsset>;
pub type DynRenderAsset = ArcHandle<dyn Any>;
pub type DynAssetLoadFn = Box<dyn Fn(LoadContext, &Path) -> DynAsset>;
pub type DynAssetWriteFn = Box<dyn Fn(&mut DynAsset, &Path)>;
pub type DynAssetOnLoadFn = Box<dyn Fn(&mut DynAsset)>;
pub type TypedAssetOnLoadFn<T> = Box<dyn Fn(&mut T)>;

pub trait Asset: Any + Send + Sync {} // TODO: is this even needed? or maybe rename

pub trait AssetLoader: Send + Sync + Clone {
    type Asset: Asset;

    fn load(&self, load_ctx: LoadContext, path: &Path) -> impl Future<Output = Self::Asset>;
}

pub trait AssetWriter: AssetLoader {
    fn write(asset: &Self::Asset, path: &Path);
}

pub trait RenderAsset: Any {} // TODO: is this even needed? or maybe rename

pub trait ConvertableRenderAsset: RenderAsset + Clone {
    type SourceAsset: Asset;
    type Error: Debug + Display;

    fn convert(
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> Result<Self, Self::Error>;
}
