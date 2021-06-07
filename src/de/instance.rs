use std::fmt;

use anyhow::Result;
use bevy::{
    ecs::{
        entity::{Entity, EntityMap},
        world::World,
    },
    utils::HashSet,
};
use rand::{prelude::ThreadRng, RngCore};
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    de::component::IdentifiedComponentSeq,
    registry::{ComponentDescriptorRegistry, PrefabDescriptor, PrefabDescriptorRegistry},
    BoxedPrefabData, PrefabInstance,
};

///////////////////////////////////////////////////////////////////////////////

struct IdValidation {
    random: ThreadRng,
    collection: HashSet<Entity>,
}

impl IdValidation {
    pub fn empty() -> Self {
        Self {
            random: rand::thread_rng(),
            collection: HashSet::default(),
        }
    }

    pub fn validate(&mut self, id: Entity) -> bool {
        self.collection.insert(id)
    }

    pub fn generate_unique(&mut self) -> Entity {
        loop {
            let id = Entity::new(self.random.next_u32());
            if self.validate(id) {
                return id;
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

enum Identifier {
    Entity,
    Prefab(PrefabDescriptor),
}

struct InstanceIdentifier<'a> {
    prefab_registry: &'a PrefabDescriptorRegistry,
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
        match registry.find_by_name(v).cloned() {
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

struct PrefabInstanceDeserializer<'a> {
    id_validation: &'a mut IdValidation,
    descriptor: PrefabDescriptor,
}

impl<'a, 'de> Visitor<'de> for PrefabInstanceDeserializer<'a> {
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
        let mut source = None;
        let mut transform = None;
        let mut parent = None;
        let mut data = None;

        let PrefabInstanceDeserializer {
            id_validation,
            descriptor,
        } = self;

        let data_seed = PrefabInstanceData { descriptor };

        while let Some(key) = access.next_key()? {
            match key {
                Field::Id => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    let temp = access.next_value()?;
                    if id_validation.validate(temp) {
                        id = Some(temp);
                    } else {
                        return Err(de::Error::custom(format!("conflicting id `{}`", temp.id())));
                    }
                }
                Field::Source => {
                    if source.is_some() {
                        return Err(de::Error::duplicate_field("source"));
                    }
                    source = Some(access.next_value()?);
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

        let id = id.unwrap_or_else(|| id_validation.generate_unique());
        let parent = parent.unwrap_or_default();
        let transform = transform.unwrap_or_default();

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
    type Value = BoxedPrefabData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrefabInstanceData { descriptor } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        (descriptor.de)(&mut deserializer).map_err(de::Error::custom)
    }
}

///////////////////////////////////////////////////////////////////////////////

const ENTITY_INSTANCE_FIELDS: &'static [&'static str] = &["id", "components"];

struct EntityInstanceDeserializer<'a> {
    id_validation: &'a mut IdValidation,
    world: &'a mut World,
    source_to_prefab: &'a mut EntityMap,
    component_registry: &'a ComponentDescriptorRegistry,
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
            id_validation,
            world,
            source_to_prefab,
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
                    let temp = access.next_value()?;
                    if id_validation.validate(temp) {
                        id = Some(temp);
                    } else {
                        return Err(de::Error::custom(format!("conflicting id `{}`", temp.id())));
                    }
                }
                Field::Components => access.next_value_seed(IdentifiedComponentSeq {
                    entity_builder: &mut entity_builder,
                    component_registry,
                })?,
            }
        }

        let id = id.unwrap_or_else(|| id_validation.generate_unique());
        source_to_prefab.insert(id, entity_builder.id());

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
    id_validation: &'a mut IdValidation,
    source_to_prefab: &'a mut EntityMap,
    world: &'a mut World,
    nested_prefabs: &'a mut Vec<PrefabInstance>,
    component_registry: &'a ComponentDescriptorRegistry,
    prefab_registry: &'a PrefabDescriptorRegistry,
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
            id_validation,
            source_to_prefab,
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
                    id_validation,
                    world,
                    source_to_prefab,
                    component_registry,
                },
            ),
            Identifier::Prefab(descriptor) => {
                nested_prefabs.push(variant.struct_variant(
                    PREFAB_INSTANCE_FIELDS,
                    PrefabInstanceDeserializer {
                        id_validation,
                        descriptor,
                    },
                )?);
                Ok(())
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) struct IdentifiedInstanceSeq<'a> {
    pub source_to_prefab: &'a mut EntityMap,
    pub world: &'a mut World,
    pub nested_prefabs: &'a mut Vec<PrefabInstance>,
    pub component_registry: &'a ComponentDescriptorRegistry,
    pub prefab_registry: &'a PrefabDescriptorRegistry,
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
            source_to_prefab,
            world,
            nested_prefabs,
            component_registry,
            prefab_registry,
        } = self;

        let id_validation = &mut IdValidation::empty();

        while let Some(_) = seq.next_element_seed(IdentifiedInstance {
            id_validation,
            source_to_prefab,
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
        fn construct(&self, world: &mut World, root: Entity) -> Result<()> {
            let _ = world;
            let _ = root;
            unimplemented!("blanket implementation")
        }
    }

    #[test]
    fn read() {
        let mut component_registry = ComponentDescriptorRegistry::default();
        component_registry
            .register_aliased::<Name>("Name".to_string())
            .unwrap();

        let mut prefab_registry = PrefabDescriptorRegistry::default();
        prefab_registry
            .register_aliased::<Lamp>("Lamp".to_string())
            .unwrap();

        let id_validation = &mut IdValidation::empty();
        let mut source_to_prefab = EntityMap::default();
        let mut world = World::default();
        let mut nested_prefabs = vec![];

        let input = r#"Lamp(
            id: 95649,
            source: External("prefabs/lamp.prefab"),
            transform: (
                position: Some((0, 0, 0)),
                rotation: Some((0, 0, 0, 1)),
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
            id_validation,
            source_to_prefab: &mut source_to_prefab,
            world: &mut world,
            nested_prefabs: &mut nested_prefabs,
            component_registry: &component_registry,
            prefab_registry: &prefab_registry,
        };
        visitor.deserialize(&mut deserializer).unwrap();

        let input = r#"Entity(
            components: [
                Name(("Root")),
            ],
        )"#;

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let visitor = IdentifiedInstance {
            id_validation,
            source_to_prefab: &mut source_to_prefab,
            world: &mut world,
            nested_prefabs: &mut nested_prefabs,
            component_registry: &component_registry,
            prefab_registry: &prefab_registry,
        };
        visitor.deserialize(&mut deserializer).unwrap();
    }
}
