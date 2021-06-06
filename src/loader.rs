use anyhow::Result;
use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    prelude::*,
};
use serde::de::DeserializeSeed;

use crate::{
    de::PrefabDeserializer,
    registry::{
        ComponentDescriptorRegistry, ComponentEntityMapperRegistry, PrefabDescriptorRegistry,
    },
};

pub struct PrefabLoader {
    asset_server: AssetServer,
    prefab_deserializer: PrefabDeserializer,
}

impl FromWorld for PrefabLoader {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap().clone();
        let entity_mapper = world
            .get_resource::<ComponentEntityMapperRegistry>()
            .unwrap();
        let component_registry = world.get_resource::<ComponentDescriptorRegistry>().unwrap();
        let prefab_registry = world.get_resource::<PrefabDescriptorRegistry>().unwrap();

        PrefabLoader {
            asset_server,
            prefab_deserializer: PrefabDeserializer::new(
                entity_mapper,
                component_registry,
                prefab_registry,
            ),
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
            let reader = self.prefab_deserializer.read();

            let prefab = self
                .asset_server
                .with_asset_refs_serialization(|| reader.deserialize(&mut deserializer))?;

            load_context.set_default_asset(LoadedAsset::new(prefab));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["prefab"]
    }
}
