use std::fmt;

use bevy::ecs::{entity::EntityMap, world::World};
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{ComponentDescriptor, PrefabDescriptor, RegistryInner},
    Prefab, PrefabInstance,
};

mod component;
mod instance;

///////////////////////////////////////////////////////////////////////////////

//use component::*;
// use prefab::*;

pub(crate) struct SceneDeserializer<'a> {
    entity_map: &'a mut EntityMap,
    components: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    prefabs: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneDeserializer<'a> {
    type Value = (World, Vec<PrefabInstance>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

pub(crate) struct PrefabDeserializer<'a> {
    components: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    prefabs: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for PrefabDeserializer<'a> {
    type Value = Prefab;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Prefab", &["variant", "scene"], self)
    }
}

impl<'a, 'de> Visitor<'de> for PrefabDeserializer<'a> {
    type Value = Prefab;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `Prafab`")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Variant,
            Scene,
        }

        let mut entity_map = EntityMap::default();
        let mut variant = None;
        let mut scene = None;

        let PrefabDeserializer {
            components,
            prefabs,
        } = self;

        while let Some(key) = access.next_key()? {
            match key {
                Field::Variant => {
                    if variant.is_some() {
                        return Err(de::Error::duplicate_field("data"));
                    }
                    variant = Some(access.next_value()?);
                }
                Field::Scene => {
                    if scene.is_some() {
                        return Err(de::Error::duplicate_field("data"));
                    }
                    scene = Some(access.next_value_seed(SceneDeserializer {
                        entity_map: &mut entity_map,
                        components,
                        prefabs,
                    })?);
                }
            }
        }

        let variant = variant.unwrap_or_default();
        let (world, nested_prefabs) = scene.ok_or(de::Error::missing_field("scene"))?;

        Ok(Prefab {
            variant,
            entity_map,
            world,
            nested_prefabs,
        })
    }
}
