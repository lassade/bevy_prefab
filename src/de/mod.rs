use std::{fmt, sync::Arc};

use bevy::{
    ecs::{entity::EntityMap, world::World},
    prelude::GlobalTransform,
    transform::components::Parent,
};
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{
        ComponentDescriptorRegistry, ComponentEntityMapperRegistry, PrefabDescriptor,
        PrefabDescriptorRegistry,
    },
    BoxedPrefabData, Prefab, PrefabConstruct, PrefabNotInstantiatedTag, PrefabTypeUuid,
};

mod component;
mod instance;

use instance::IdentifiedInstanceSeq;

///////////////////////////////////////////////////////////////////////////////

struct PrefabVariant<'a> {
    prefab_registry: &'a PrefabDescriptorRegistry,
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

struct PrefabDataDeserializer {
    descriptor: PrefabDescriptor,
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
    component_entity_mapper: &'a ComponentEntityMapperRegistry,
    component_registry: &'a ComponentDescriptorRegistry,
    prefab_registry: &'a PrefabDescriptorRegistry,
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
            component_entity_mapper,
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
                Field::Transform => {
                    if transform.is_some() {
                        return Err(de::Error::duplicate_field("transform"));
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
                nested.source.clone().unwrap_or_default(),
                GlobalTransform::default(),
                nested.transform.clone(),
                PrefabNotInstantiatedTag { _marker: () },
            ));

            if nested.source.is_none() {
                // Source isn't available, insert construct function definition
                blank.insert(PrefabConstruct(nested.constructor));
            } else {
                // Validate source type
                blank.insert(PrefabTypeUuid(nested.uuid));
            }

            if let Some(prefab_data) = &nested.data {
                // Insert the PrefabData (down casted) in the root Entity so it can be available during runtime
                prefab_data.0.copy_to_instance(&mut blank);
            }
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
        component_entity_mapper
            .map_world_components(&mut world, &source_to_prefab)
            .map_err(de::Error::custom)?;

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

pub(crate) struct PrefabDeserializerInner {
    pub component_entity_mapper: ComponentEntityMapperRegistry,
    pub component_registry: ComponentDescriptorRegistry,
    pub prefab_registry: PrefabDescriptorRegistry,
}

#[derive(Clone)]
pub(crate) struct PrefabDeserializer {
    pub inner: Arc<PrefabDeserializerInner>,
}

impl PrefabDeserializer {
    pub fn new(
        component_entity_mapper: ComponentEntityMapperRegistry,
        component_registry: ComponentDescriptorRegistry,
        prefab_registry: PrefabDescriptorRegistry,
    ) -> Self {
        Self {
            inner: Arc::new(PrefabDeserializerInner {
                component_entity_mapper,
                component_registry,
                prefab_registry,
            }),
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabDeserializer {
    type Value = Prefab;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Prefab", &[], self)
    }
}

impl<'a, 'de> Visitor<'de> for &'a PrefabDeserializer {
    type Value = Prefab;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let PrefabDeserializerInner {
            component_entity_mapper,
            component_registry,
            prefab_registry,
        } = &*self.inner;

        let (descriptor, variant) = data.variant_seed(PrefabVariant { prefab_registry })?;
        variant.struct_variant(
            PREFAB_FIELDS,
            PrefabBody {
                descriptor,
                component_entity_mapper,
                component_registry,
                prefab_registry,
            },
        )
    }
}
