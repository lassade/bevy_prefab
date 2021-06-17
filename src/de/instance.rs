use std::fmt;

use anyhow::Result;
use bevy::{
    ecs::{
        entity::{Entity, EntityMap},
        world::World,
    },
    prelude::{Handle, Parent},
    utils::HashSet,
};
use rand::{prelude::ThreadRng, RngCore};
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    data::BoxedPrefabOverrides,
    de::component::IdentifiedComponentSeq,
    registry::{ComponentDescriptorRegistry, PrefabDescriptor, PrefabDescriptorRegistry},
    Prefab, PrefabConstruct, PrefabNotInstantiatedTag, PrefabTransformOverride, PrefabTypeUuid,
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
    world: &'a mut World,
    source_to_prefab: &'a mut EntityMap,
    descriptor: PrefabDescriptor,
    component_registry: &'a ComponentDescriptorRegistry,
}

impl<'a, 'de> Visitor<'de> for PrefabInstanceDeserializer<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `Prefab` instance")
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
            Parent,
            Transform,
            Overrides,
            Components,
        }

        let mut id = None;
        let mut source: Option<Handle<Prefab>> = None;
        let mut transform_override = None;
        let mut parent = None;
        let mut overrides = None;

        let PrefabInstanceDeserializer {
            id_validation,
            world,
            source_to_prefab,
            descriptor,
            component_registry,
        } = self;

        let data_seed = PrefabInstanceDataOverrides { descriptor };

        // spawn nested prefab instance entity
        let mut prefab_entity = world.spawn();

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
                Field::Parent => {
                    if parent.is_some() {
                        return Err(de::Error::duplicate_field("parent"));
                    }
                    parent = Some(access.next_value()?);
                }
                Field::Transform => {
                    if transform_override.is_some() {
                        return Err(de::Error::duplicate_field("transform"));
                    }
                    transform_override = Some(access.next_value()?);
                }
                Field::Overrides => {
                    if overrides.is_some() {
                        return Err(de::Error::duplicate_field("overrides"));
                    }
                    overrides = Some(access.next_value_seed(&data_seed)?);
                }
                Field::Components => access.next_value_seed(IdentifiedComponentSeq {
                    entity_builder: &mut prefab_entity,
                    component_registry,
                })?,
            }
        }

        // prefabs only driven by code doesn't need source prefabs to define their main,
        // here checks if the prefab needs the source field or not and give error to the user
        if data_seed.descriptor.source_prefab_required {
            if source.is_none() {
                Err(de::Error::missing_field("source"))?;
            }
        } else {
            if source.is_some() {
                Err(de::Error::custom("source isn't used by prefab"))?;
            }
        }

        let id = id.unwrap_or_else(|| id_validation.generate_unique());
        let parent = parent.unwrap_or_default();
        let transform_override: PrefabTransformOverride = transform_override.unwrap_or_default();

        let blank_entity = prefab_entity.id();
        source_to_prefab.insert(id, blank_entity);

        prefab_entity.insert_bundle((
            source.clone().unwrap_or_default(),
            transform_override,
            PrefabNotInstantiatedTag { _marker: () },
        ));

        if !data_seed.descriptor.source_prefab_required {
            // source isn't available, insert construct function definition
            prefab_entity.insert(PrefabConstruct(data_seed.descriptor.construct));
        } else {
            // validate source type
            prefab_entity.insert(PrefabTypeUuid(data_seed.descriptor.uuid));
        }

        if let Some(overrides) = overrides {
            prefab_entity.insert(overrides);
        }

        // parent all nested prefabs (when needed)
        if let Some(source_parent) = parent {
            // NOTE here we don't convert the `source_parent` entity because
            // it will be done at in the next stage of deserialization
            prefab_entity.insert(Parent(source_parent));
        }

        Ok(())
    }
}

struct PrefabInstanceDataOverrides {
    descriptor: PrefabDescriptor,
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabInstanceDataOverrides {
    type Value = BoxedPrefabOverrides;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrefabInstanceDataOverrides { descriptor } = self;
        descriptor
            .overrides
            .deserialize(deserializer)
            .map_err(de::Error::custom)
            .map(BoxedPrefabOverrides)
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
            Identifier::Prefab(descriptor) => variant.struct_variant(
                PREFAB_INSTANCE_FIELDS,
                PrefabInstanceDeserializer {
                    id_validation,
                    world,
                    source_to_prefab,
                    descriptor,
                    component_registry,
                },
            ),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) struct IdentifiedInstanceSeq<'a> {
    pub source_to_prefab: &'a mut EntityMap,
    pub world: &'a mut World,
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
            component_registry,
            prefab_registry,
        } = self;

        let id_validation = &mut IdValidation::empty();

        while let Some(_) = seq.next_element_seed(IdentifiedInstance {
            id_validation,
            source_to_prefab,
            world,
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
    use bevy::{
        ecs::{
            entity::{MapEntities, MapEntitiesError},
            world::World,
        },
        reflect::{Reflect, TypeUuid},
    };
    use serde::Deserialize;

    use super::*;
    use crate::{
        registry::{ComponentDescriptorRegistry, PrefabDescriptorRegistry},
        PrefabData,
    };

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone, Reflect)]
    struct Name(String);

    #[derive(Default, Debug, Deserialize, Clone, TypeUuid, Reflect)]
    #[uuid = "8c24e0d1-98cc-4865-b27a-c776f5ba614d"]
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

    // TODO: remove
    impl MapEntities for Lamp {
        fn map_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
            Ok(())
        }
    }

    #[test]
    fn read() {
        let mut component_registry = ComponentDescriptorRegistry::default();
        component_registry
            .register::<Name>("Name".to_string())
            .unwrap();

        let mut prefab_registry = PrefabDescriptorRegistry::default();
        prefab_registry
            .register_aliased::<Lamp>("Lamp".to_string(), true)
            .unwrap();

        let id_validation = &mut IdValidation::empty();
        let mut source_to_prefab = EntityMap::default();
        let mut world = World::default();

        let input = r#"Lamp(
            id: 95649,
            source: External("prefabs/lamp.prefab"),
            transform: (
                position: Some((0, 0, 0)),
                rotation: Some((0, 0, 0, 1)),
                scale: None,
            ),
            parent: Some(67234),
            overrides: (
                //light_color: LinRgba(1, 0, 0, 1),
                light_strength: 2,
            ),
        )"#;

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let visitor = IdentifiedInstance {
            id_validation,
            source_to_prefab: &mut source_to_prefab,
            world: &mut world,
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
            component_registry: &component_registry,
            prefab_registry: &prefab_registry,
        };
        visitor.deserialize(&mut deserializer).unwrap();
    }
}
