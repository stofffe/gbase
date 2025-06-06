use crate::render::{self, ArcHandle};
use std::{
    any::Any,
    fmt::{Debug, Display},
    path::Path,
};

pub type DynAsset = Box<dyn Asset>;
pub type DynRenderAsset = ArcHandle<dyn Any + Send + Sync>;
pub type DynAssetLoadFn = Box<dyn Fn(&Path) -> DynAsset + Send + Sync>;
pub type DynAssetWriteFn = Box<dyn Fn(&mut DynAsset, &Path)>;
pub type DynAssetOnLoadFn = Box<dyn Fn(&mut DynAsset) + Send + Sync>;
pub type TypedAssetOnLoadFn<T> = Box<dyn Fn(&mut T) + Send + Sync>;

pub trait Asset: Any + Send + Sync {}

pub trait LoadableAsset: Asset {
    fn load(path: &Path) -> Self;
}
pub trait WriteableAsset: LoadableAsset {
    fn write(&mut self, path: &Path);
}

pub trait RenderAsset: Any {}

pub trait ConvertableRenderAsset: RenderAsset + Send + Sync + Sized {
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
