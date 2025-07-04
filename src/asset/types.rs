use super::AssetHandle;
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
pub type DynAssetLoadFn = Box<dyn Fn(&Path) -> DynAsset>;
pub type DynAssetWriteFn = Box<dyn Fn(&mut DynAsset, &Path)>;
pub type DynAssetOnLoadFn = Box<dyn Fn(&mut DynAsset)>;
pub type TypedAssetOnLoadFn<T> = Box<dyn Fn(&mut T)>;

pub trait Asset: Any + Send + Sync {}

pub trait LoadableAsset: Asset {
    fn load(path: &Path) -> impl Future<Output = Self>;
}
pub trait WriteableAsset: LoadableAsset {
    fn write(&mut self, path: &Path);
}

pub trait RenderAsset: Any {} // TODO: is this even needed? or maybe rename

// pub trait ConvertableRenderAsset: RenderAsset + Sync + Sized {
pub trait ConvertableRenderAsset: RenderAsset + Sized + Clone {
    type SourceAsset: Asset;
    type Params;
    type Error: Debug + Display;

    fn convert(
        ctx: &mut Context,
        source: &Self::SourceAsset,
        params: &Self::Params,
    ) -> Result<Self, Self::Error>;
}
