use crate::render::{self, ArcHandle};
use std::{
    any::Any,
    fmt::{Debug, Display},
    future::Future,
    path::Path,
    pin::Pin,
};

pub type DynAsset = Box<dyn Asset>;
pub type DynRenderAsset = ArcHandle<dyn Any>;
pub type DynAssetLoadFn = Box<dyn Fn(&Path) -> DynAsset>;
// pub type DynAssetLoadFn =
//     Box<dyn Fn(&Path) -> Pin<Box<dyn Future<Output = DynAsset> + Send>> + Send + Sync>;
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

pub trait RenderAsset: Any {}

// pub trait ConvertableRenderAsset: RenderAsset + Sync + Sized {
pub trait ConvertableRenderAsset: RenderAsset + Sized + Clone {
    type SourceAsset: Asset;
    type Params;
    type Error: Debug + Display;

    fn convert(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_cache: &mut render::RenderCache,
        source: &Self::SourceAsset,
        params: &Self::Params,
    ) -> Result<Self, Self::Error>;
}
