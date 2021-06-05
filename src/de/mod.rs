use std::fmt;

use bevy::ecs::{entity::EntityMap, world::World};
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{
        ComponentDescriptor, ComponentDescriptorRegistry, PrefabDescriptor,
        PrefabDescriptorRegistry, RegistryInner,
    },
    Prefab,
};

mod component;
mod instance;

use instance::IdentifiedInstanceSeq;

///////////////////////////////////////////////////////////////////////////////

pub struct PrefabDeserializer<'a> {
    component_registry: RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    prefab_registry: RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a> PrefabDeserializer<'a> {
    pub fn new(
        component_registry: &'a ComponentDescriptorRegistry,
        prefab_registry: &'a PrefabDescriptorRegistry,
    ) -> Self {
        Self {
            component_registry: component_registry.lock.read(),
            prefab_registry: prefab_registry.lock.read(),
        }
    }
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
        let mut world = World::default();
        let mut nested_prefabs = vec![];

        let PrefabDeserializer {
            component_registry,
            prefab_registry,
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
                    access.next_value_seed(IdentifiedInstanceSeq {
                        entity_map: &mut entity_map,
                        world: &mut world,
                        nested_prefabs: &mut nested_prefabs,
                        component_registry: &component_registry,
                        prefab_registry: &prefab_registry,
                    })?;
                }
            }
        }

        let variant = variant.unwrap_or_default();
        Ok(Prefab {
            variant,
            entity_map,
            world,
            nested_prefabs,
        })
    }
}
