use std::fmt;

use bevy::{
    ecs::{entity::EntityMap, world::World},
    transform::components::Parent,
};
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{
        ComponentDescriptor, ComponentDescriptorRegistry, ComponentEntityMapperRegistry,
        ComponentEntityMapperRegistryInner, PrefabDescriptor, PrefabDescriptorRegistry,
        RegistryInner,
    },
    BoxedPrefabData, Prefab, PrefabConstruct, PrefabNotInstantiatedTag,
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
        match registry.find_by_name(v).cloned() {
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
    entity_mapper: &'a RwLockReadGuard<'a, ComponentEntityMapperRegistryInner>,
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
            Transform,
            Scene,
        }

        let mut source_to_prefab = EntityMap::default();
        let mut defaults = None;
        let mut transform = None;
        let mut world = World::default();
        let mut nested_prefabs = vec![];

        let PrefabBody {
            entity_mapper,
            descriptor,
            component_registry,
            prefab_registry,
        } = self;

        let construct = descriptor.construct.clone();
        let data_seed = PrefabDataDeserializer { descriptor };

        while let Some(key) = access.next_key()? {
            match key {
                Field::Defaults => {
                    if defaults.is_some() {
                        return Err(de::Error::duplicate_field("defaults"));
                    }
                    defaults = Some(access.next_value_seed(&data_seed)?);
                }
                Field::Transform => {
                    if transform.is_some() {
                        return Err(de::Error::duplicate_field("defaults"));
                    }
                    transform = Some(access.next_value()?);
                }
                Field::Scene => {
                    access.next_value_seed(IdentifiedInstanceSeq {
                        source_to_prefab: &mut source_to_prefab,
                        world: &mut world,
                        nested_prefabs: &mut nested_prefabs,
                        component_registry,
                        prefab_registry,
                    })?;
                }
            }
        }

        // Spawn blank nested prefab instances
        for nested in &mut nested_prefabs {
            let mut blank = world.spawn();
            let blank_entity = blank.id();
            source_to_prefab.insert(nested.id, blank_entity);

            // Map prefab entity id from source to prefab space (required for the next step)
            nested.id = blank_entity;

            blank.insert_bundle((
                nested.source.clone(),
                nested.transform.clone(),
                PrefabNotInstantiatedTag,
                PrefabConstruct(construct),
            ));

            let prefab_data = &nested.data.0;
            // Insert the PrefabData (down casted) in the root Entity so it can be available during runtime
            prefab_data.copy_to_instance(&mut blank);
        }

        // Parent all nested prefabs (when needed)
        for nested in &mut nested_prefabs {
            if let Some(source_parent) = nested.parent {
                let prefab_parent = source_to_prefab
                    .get(source_parent)
                    .map_err(de::Error::custom)?;
                world.entity_mut(nested.id).insert(Parent(prefab_parent));
            }
        }

        // Map entities from source file to prefab space
        entity_mapper.map_world_components(&mut world, &source_to_prefab);

        let defaults = defaults.unwrap_or_else(|| (data_seed.descriptor.default)());
        let transform = transform.unwrap_or_default();
        Ok(Prefab {
            defaults,
            transform,
            world,
        })
    }
}

///////////////////////////////////////////////////////////////////////////////

const PREFAB_FIELDS: &'static [&'static str] = &["defaults", "scene"];

pub struct PrefabDeserializer<'a> {
    entity_mapper: RwLockReadGuard<'a, ComponentEntityMapperRegistryInner>,
    component_registry: RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    prefab_registry: RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a> PrefabDeserializer<'a> {
    pub fn new(
        entity_mapper: &'a ComponentEntityMapperRegistry,
        component_registry: &'a ComponentDescriptorRegistry,
        prefab_registry: &'a PrefabDescriptorRegistry,
    ) -> Self {
        Self {
            entity_mapper: entity_mapper.lock.read(),
            component_registry: component_registry.lock.read(),
            prefab_registry: prefab_registry.lock.read(),
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabDeserializer<'a> {
    type Value = Prefab;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Prefab", &[], self)
    }
}

impl<'a, 'de> Visitor<'de> for &'a PrefabDeserializer<'a> {
    type Value = Prefab;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let PrefabDeserializer {
            entity_mapper,
            component_registry,
            prefab_registry,
        } = self;

        let (descriptor, variant) = data.variant_seed(PrefabVariant { prefab_registry })?;
        variant.struct_variant(
            PREFAB_FIELDS,
            PrefabBody {
                descriptor,
                entity_mapper,
                component_registry,
                prefab_registry,
            },
        )
    }
}
