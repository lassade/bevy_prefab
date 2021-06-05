use std::fmt;

use bevy::ecs::{entity::EntityMap, world::World};
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{
        ComponentDescriptor, ComponentDescriptorRegistry, PrefabDescriptor,
        PrefabDescriptorRegistry, RegistryInner,
    },
    BoxedPrefabData, Prefab,
};

mod component;
mod instance;

use instance::IdentifiedInstanceSeq;

///////////////////////////////////////////////////////////////////////////////

pub(crate) struct PrefabVariant<'a> {
    pub prefab_registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for PrefabVariant<'a> {
    type Value = PrefabDescriptor;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'a, 'de> Visitor<'de> for PrefabVariant<'a> {
    type Value = PrefabDescriptor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab` identifier")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let PrefabVariant {
            prefab_registry: registry,
        } = self;
        match registry.named.get(v).cloned() {
            Some(descriptor) => Ok(descriptor),
            None => Err(de::Error::unknown_variant(v, &[])),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
pub struct PrefabDataDeserializer {
    pub descriptor: PrefabDescriptor,
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabDataDeserializer {
    type Value = BoxedPrefabData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrefabDataDeserializer { descriptor } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        (descriptor.de)(&mut deserializer).map_err(de::Error::custom)
    }
}

///////////////////////////////////////////////////////////////////////////////

struct PrefabBody<'a> {
    descriptor: PrefabDescriptor,
    component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    prefab_registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> Visitor<'de> for PrefabBody<'a> {
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
            Defaults,
            Scene,
        }

        let mut entity_map = EntityMap::default();
        let mut defaults = None;
        let mut world = World::default();
        let mut nested_prefabs = vec![];

        let PrefabBody {
            descriptor,
            component_registry,
            prefab_registry,
        } = self;

        let data_seed = PrefabDataDeserializer { descriptor };

        while let Some(key) = access.next_key()? {
            match key {
                Field::Defaults => {
                    if defaults.is_some() {
                        return Err(de::Error::duplicate_field("defaults"));
                    }
                    defaults = Some(access.next_value_seed(&data_seed)?);
                }
                Field::Scene => {
                    access.next_value_seed(IdentifiedInstanceSeq {
                        entity_map: &mut entity_map,
                        world: &mut world,
                        nested_prefabs: &mut nested_prefabs,
                        component_registry,
                        prefab_registry,
                    })?;
                }
            }
        }

        let defaults = defaults.unwrap_or_else(|| (data_seed.descriptor.default)());
        Ok(Prefab {
            defaults,
            entity_map,
            world,
            nested_prefabs,
        })
    }
}

///////////////////////////////////////////////////////////////////////////////

const PREFAB_FIELDS: &'static [&'static str] = &["defaults", "scene"];

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
        deserializer.deserialize_enum("Prefab", &[], self)
    }
}

impl<'a, 'de> Visitor<'de> for PrefabDeserializer<'a> {
    type Value = Prefab;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let PrefabDeserializer {
            component_registry,
            prefab_registry,
        } = &self;

        let (descriptor, variant) = data.variant_seed(PrefabVariant { prefab_registry })?;
        variant.struct_variant(
            PREFAB_FIELDS,
            PrefabBody {
                descriptor,
                component_registry,
                prefab_registry,
            },
        )
    }
}
