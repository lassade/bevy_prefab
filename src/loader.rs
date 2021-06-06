use anyhow::Result;
use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    prelude::*,
};
use serde::de::DeserializeSeed;

use crate::de::PrefabDeserializer;

///////////////////////////////////////////////////////////////////////////////

pub struct PrefabLoader {
    asset_server: AssetServer,
    prefab_deserializer: PrefabDeserializer,
}

impl FromWorld for PrefabLoader {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap().clone();
        let prefab_deserializer = world.get_resource::<PrefabDeserializer>().unwrap().clone();
        PrefabLoader {
            asset_server,
            prefab_deserializer,
        }
    }
}

impl AssetLoader for PrefabLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
            let prefab = self.asset_server.with_asset_refs_serialization(|| {
                // ? NOTE: Keep this scope as lean as possible
                self.prefab_deserializer.deserialize(&mut deserializer)
            })?;
            load_context.set_default_asset(LoadedAsset::new(prefab));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["prefab"]
    }
}
