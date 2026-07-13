use super::DynAsset;
use crate::{
    asset::{self, Asset, AssetCache, AssetConverter, ConvertAssetResult, GetAssetResult},
    Context,
};
use std::{marker::PhantomData, sync::Arc};

#[derive(Debug)]
pub struct AssetHandle<T: 'static> {
    pub(crate) id: Arc<u64>, // TODO: use strong and weak outside/inside cache
    pub(crate) ty: PhantomData<T>,
}

impl<T: 'static> AssetHandle<T> {
    #![allow(clippy::new_without_default)]
    pub fn new(asset_handle_ctx: &asset::AssetHandleContext) -> Self {
        let id = asset_handle_ctx.next_id();
        Self {
            id: Arc::new(id),
            ty: PhantomData,
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        *self.id
    }

    pub(crate) fn as_any(&self) -> AssetHandle<DynAsset> {
        AssetHandle::<DynAsset> {
            id: self.id.clone(),
            ty: PhantomData,
        }
    }
}

impl<T: 'static> PartialOrd for AssetHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: 'static> Ord for AssetHandle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: 'static> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Eq for AssetHandle<T> {}

impl<T: 'static> std::hash::Hash for AssetHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: 'static> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            ty: PhantomData,
        }
    }
}

impl<T: Asset + 'static> AssetHandle<T> {
    pub fn loaded(&self, cache: &AssetCache) -> bool {
        cache.handle_loaded(self.clone())
    }

    pub fn just_loaded(&self, cache: &AssetCache) -> bool {
        cache.handle_just_loaded(self.clone())
    }

    pub fn get<'a>(&self, cache: &'a mut AssetCache) -> GetAssetResult<'a, T> {
        cache.get(self.clone())
    }

    pub fn convert_custom_settings<G: AssetConverter<SourceAsset = T>>(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        cache.convert::<G>(ctx, self.clone(), settings)
    }

    pub fn convert_default_settings<G: AssetConverter<SourceAsset = T, Settings: Default>>(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
    ) -> ConvertAssetResult<G::TargetAsset> {
        cache.convert::<G>(ctx, self.clone(), &G::Settings::default())
    }
}
