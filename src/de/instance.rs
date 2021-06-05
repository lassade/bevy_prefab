use std::fmt;

use anyhow::Result;
use bevy::ecs::{
    entity::{Entity, EntityMap},
    world::World,
};
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    de::component::IdentifiedComponentSeq,
    registry::{ComponentDescriptor, PrefabDescriptor, RegistryInner},
    BoxedPrefabData, PrefabInstance,
};

///////////////////////////////////////////////////////////////////////////////

enum Identifier {
    Entity,
    Prefab(PrefabDescriptor),
}

struct InstanceIdentifier<'a> {
    prefab_registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for InstanceIdentifier<'a> {
    type Value = Identifier;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'a, 'de> Visitor<'de> for InstanceIdentifier<'a> {
    type Value = Identifier;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab` or a plain `Entity` identifier")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let InstanceIdentifier {
            prefab_registry: registry,
        } = self;
        match registry.named.get(v).cloned() {
            Some(descriptor) => Ok(Identifier::Prefab(descriptor)),
            None => {
                // Plain entity
                if v == "Entity" {
                    Ok(Identifier::Entity)
                } else {
                    return Err(de::Error::unknown_variant(v, &[]));
                }
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

const PREFAB_INSTANCE_FIELDS: &'static [&'static str] =
    &["id", "source", "transform", "parent", "data"];

struct PrefabInstanceDeserializer {
    descriptor: PrefabDescriptor,
}

impl<'de> Visitor<'de> for PrefabInstanceDeserializer {
    type Value = PrefabInstance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `PrefabInstance` struct")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Source,
            Transform,
            Parent,
            Data,
        }

        let mut id = None;
        let source = None;
        let mut transform = None;
        let mut parent = None;
        let mut data = None;

        let PrefabInstanceDeserializer { descriptor } = self;
        let data_seed = PrefabInstanceData { descriptor };

        while let Some(key) = access.next_key()? {
            match key {
                Field::Id => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    id = Some(access.next_value()?);
                }
                Field::Source => {
                    // TODO:
                    // if source.is_some() {
                    //     return Err(de::Error::duplicate_field("source"));
                    // }
                    // source = Some(access.next_value()?);
                }
                Field::Transform => {
                    if transform.is_some() {
                        return Err(de::Error::duplicate_field("transform"));
                    }
                    transform = Some(access.next_value()?);
                }
                Field::Parent => {
                    if parent.is_some() {
                        return Err(de::Error::duplicate_field("parent"));
                    }
                    parent = Some(access.next_value()?);
                }
                Field::Data => {
                    if data.is_some() {
                        return Err(de::Error::duplicate_field("data"));
                    }
                    data = Some(access.next_value_seed(&data_seed)?);
                }
            }
        }

        let id = id.ok_or(de::Error::missing_field("id"))?;
        let source = source.unwrap_or_default(); // TODO: Should return error on missing field
        let parent = parent.unwrap_or_default();
        let transform = transform.unwrap_or_default();
        let data = data.unwrap_or_default(); // TODO: Create prefab default data

        Ok(PrefabInstance {
            id,
            source,
            parent,
            transform,
            data,
        })
    }
}

struct PrefabInstanceData {
    descriptor: PrefabDescriptor,
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabInstanceData {
    type Value = Option<BoxedPrefabData>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrefabInstanceData { descriptor } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        let data = (descriptor.de)(&mut deserializer).map_err(de::Error::custom)?;
        Ok(Some(data))
    }
}

///////////////////////////////////////////////////////////////////////////////

const ENTITY_INSTANCE_FIELDS: &'static [&'static str] = &["id", "components"];

struct EntityInstanceDeserializer<'a> {
    world: &'a mut World,
    entity_map: &'a mut EntityMap,
    component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
}

impl<'a, 'de> Visitor<'de> for EntityInstanceDeserializer<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `PrefabInstance`")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Components,
        }

        let EntityInstanceDeserializer {
            world,
            entity_map,
            component_registry,
        } = self;

        let mut entity_builder = world.spawn();
        let mut id = None;

        while let Some(key) = access.next_key()? {
            match key {
                Field::Id => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    id = Some(access.next_value::<Entity>()?);
                }
                Field::Components => access.next_value_seed(IdentifiedComponentSeq {
                    entity_builder: &mut entity_builder,
                    component_registry,
                })?,
            }
        }

        let id = id.ok_or(de::Error::missing_field("id"))?;
        entity_map.insert(id, entity_builder.id());

        Ok(())
    }
}

impl<'a, 'de> DeserializeSeed<'de> for EntityInstanceDeserializer<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Entity", ENTITY_INSTANCE_FIELDS, self)
    }
}

///////////////////////////////////////////////////////////////////////////////

struct IdentifiedInstance<'a> {
    entity_map: &'a mut EntityMap,
    world: &'a mut World,
    nested_prefabs: &'a mut Vec<PrefabInstance>,
    component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    prefab_registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for IdentifiedInstance<'a> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Prefab", &[], self)
    }
}

impl<'a, 'de> Visitor<'de> for IdentifiedInstance<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab` or a plain `Entity` instance")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let IdentifiedInstance {
            entity_map,
            world,
            nested_prefabs,
            component_registry,
            prefab_registry,
        } = self;

        let (instance, variant) = data.variant_seed(InstanceIdentifier { prefab_registry })?;

        match instance {
            Identifier::Entity => variant.struct_variant(
                ENTITY_INSTANCE_FIELDS,
                EntityInstanceDeserializer {
                    world,
                    entity_map,
                    component_registry,
                },
            ),
            Identifier::Prefab(descriptor) => {
                nested_prefabs.push(variant.struct_variant(
                    PREFAB_INSTANCE_FIELDS,
                    PrefabInstanceDeserializer { descriptor },
                )?);
                Ok(())
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) struct IdentifiedInstanceSeq<'a> {
    pub entity_map: &'a mut EntityMap,
    pub world: &'a mut World,
    pub nested_prefabs: &'a mut Vec<PrefabInstance>,
    pub component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
    pub prefab_registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for IdentifiedInstanceSeq<'a> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'a, 'de> Visitor<'de> for IdentifiedInstanceSeq<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `Prefab` or `Entity` sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let IdentifiedInstanceSeq {
            entity_map,
            world,
            nested_prefabs,
            component_registry,
            prefab_registry,
        } = self;

        while let Some(_) = seq.next_element_seed(IdentifiedInstance {
            entity_map,
            world,
            nested_prefabs,
            component_registry,
            prefab_registry,
        })? {
            // Do nothing, just deserialize all elements in the sequence
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;
    use serde::Deserialize;

    use super::*;
    use crate::{
        registry::{ComponentDescriptorRegistry, PrefabDescriptorRegistry},
        PrefabData,
    };

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    struct Name(String);

    #[derive(Default, Debug, Deserialize, Clone)]
    struct Lamp {
        light_strength: f32,
    }

    impl PrefabData for Lamp {
        fn construct(&self, world: &mut World) -> Result<()> {
            let _ = world;
            todo!()
        }
    }

    #[test]
    fn read() {
        let component_registry = ComponentDescriptorRegistry::default();
        component_registry.register::<Name>().unwrap();

        let prefab_registry = PrefabDescriptorRegistry::default();
        prefab_registry.register::<Lamp>().unwrap();

        let mut entity_map = EntityMap::default();
        let mut world = World::default();
        let mut nested_prefabs = vec![];

        let input = r#"Lamp(
            id: 95649,
            //source: (
            //    uuid: "76500818-9b39-4655-9d32-8f1ac0ecbb41",
            //    path: "prefabs/lamp.prefab",
            //),
            transform: (
                position: (0, 0, 0),
                rotation: (0, 0, 0, 1),
                scale: None,
            ),
            parent: Some(67234),
            data: (
                //light_color: LinRgba(1, 0, 0, 1),
                light_strength: 2,
            ),
        )"#;

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let visitor = IdentifiedInstance {
            entity_map: &mut entity_map,
            world: &mut world,
            nested_prefabs: &mut nested_prefabs,
            component_registry: &component_registry.lock.read(),
            prefab_registry: &prefab_registry.lock.read(),
        };
        visitor.deserialize(&mut deserializer).unwrap();

        let input = r#"Entity(
            id: 95649,
            components: [
                Name(("Root")),
            ],
        )"#;

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let visitor = IdentifiedInstance {
            entity_map: &mut entity_map,
            world: &mut world,
            nested_prefabs: &mut nested_prefabs,
            component_registry: &component_registry.lock.read(),
            prefab_registry: &prefab_registry.lock.read(),
        };
        visitor.deserialize(&mut deserializer).unwrap();
    }
}
