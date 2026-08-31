use std::num::NonZeroU16;

use gbase::{
    asset::{Asset, AssetHandle, AssetLoader, EmptyError},
    tracing, wgpu, CallbackResult, Callbacks, Context,
};

//
// This project is used to see what types are safe to send over dll
//
// crates which are not
// * hashbrown
//

pub fn main() {
    gbase::run::<App>();
}

pub struct HashMapLoader {}

pub struct MyHashMapAsset {
    hashmap: hashbrown::HashMap<char, NonZeroU16>,
}

impl Asset for MyHashMapAsset {}

impl AssetLoader for HashMapLoader {
    type Asset = MyHashMapAsset;

    type Settings = ();

    type Error = EmptyError;

    async fn load(
        load_ctx: &mut gbase::asset::LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let mut hashmap = hashbrown::HashMap::new();
        hashmap.insert('a', NonZeroU16::new(2).expect("aa"));
        hashmap.insert('x', NonZeroU16::new(8).expect("aa"));

        Ok(MyHashMapAsset { hashmap })
    }
}

struct App {
    hashmap_handle: AssetHandle<MyHashMapAsset>,
}

impl Callbacks for App {
    #[no_mangle]
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let hashmap_handle = _cache.load_asset::<HashMapLoader>(&());
        Self { hashmap_handle }
    }

    #[no_mangle]
    fn render(
        &mut self,
        _ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        if let Ok(hashmap) = _cache.get_asset(&self.hashmap_handle) {
            tracing::info!("HashMap {:?}", hashmap.hashmap);
            tracing::info!(
                "Lookup {} from HashMap gives {:?}",
                'a',
                hashmap.hashmap.get(&'a')
            );
            tracing::info!(
                "Lookup {} from HashMap gives {:?}",
                'x',
                hashmap.hashmap.get(&'x')
            );
        }
        CallbackResult::Continue
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
