use std::{any::Any, path::Path};

use crate::render::{self, ArcHandle};

pub type DynAsset = Box<dyn Asset>;
pub type DynRenderAsset = ArcHandle<dyn Any + Send + Sync>;
pub type DynAssetLoadFn = Box<dyn Fn(&Path) -> DynAsset>;
pub type DynAssetWriteFn = Box<dyn Fn(&mut DynAsset, &Path)>;

pub trait Asset: Any + Send + Sync {}

pub trait LoadableAsset: Asset {
    fn load(path: &Path) -> Self;
}
pub trait WriteableAsset: LoadableAsset {
    fn write(&mut self, _path: &Path);
}

pub trait RenderAsset: Any {}

pub trait ConvertableRenderAsset: RenderAsset + Send + Sync {
    type SourceAsset: Asset;
    type Params;

    fn convert(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_cache: &mut render::RenderCache,
        source: &Self::SourceAsset,
        params: &Self::Params,
    ) -> Self;
}

impl dyn Asset {
    pub(crate) fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
