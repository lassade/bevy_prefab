use std::{any::Any, collections::hash_map::Entry, fmt::Debug};

use anyhow::Result;
use bevy::{
    ecs::{
        component::Component,
        entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
        world::{EntityMut, World},
    },
    reflect::{Struct, TypeUuid, Uuid},
    utils::HashMap,
};
use serde::{
    de::{self, DeserializeSeed, MapAccess},
    Deserialize, Serialize,
};

///////////////////////////////////////////////////////////////////////////////

pub trait PrefabData: PrefabDataHelper + Debug {
    /// Construct function called once on spawn
    fn construct(&self, world: &mut World, root: Entity) -> Result<()>;
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub(crate) Box<dyn PrefabData + Send + Sync>);

///////////////////////////////////////////////////////////////////////////////

/// Helper default functions
pub trait PrefabDataHelper {
    /// Copies it self in the prefab instance so that self will be available during runtime,
    /// but doesn't override the previously if already has
    fn copy_to_instance(&self, instance: &mut EntityMut);

    /// Constructs prefabs using the instance data or default to this data
    fn construct_instance(&self, world: &mut World, root: Entity) -> Result<()>;

    /// Uuid from [`TypeUuid`]
    fn type_uuid(&self) -> Uuid;
}

impl<T> PrefabDataHelper for T
where
    T: PrefabData + TypeUuid + Clone + Send + Sync + Component + 'static,
{
    fn copy_to_instance(&self, entity: &mut EntityMut) {
        if !entity.contains::<T>() {
            entity.insert(self.clone());
        }
    }

    fn construct_instance(&self, world: &mut World, root: Entity) -> Result<()> {
        // TODO: quite bit of cloning is required, maybe there's a better ways but I digress
        let mut entity = world.entity_mut(root);
        if let Some(data) = entity.get::<T>() {
            // use the prefab component data to run the construct function
            data.clone().construct(world, root)
        } else {
            // insert missing prefab data component
            entity.insert(self.clone());
            // run the construct function using the original copy of the data,
            // this data could be `Default::default` or the data from the source prefab
            self.construct(world, root)
        }
    }

    fn type_uuid(&self) -> Uuid {
        T::TYPE_UUID
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, TypeUuid)]
#[uuid = "3c603f24-9a89-45c3-8f4a-087a28f006df"]
pub struct BlankPrefab;

impl PrefabData for BlankPrefab {
    fn construct(&self, _: &mut World, _: Entity) -> Result<()> {
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////

pub trait FieldOverride {
    fn apply(&self, target: &mut dyn Any);
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError>;
}

pub(crate) struct FieldOverrideDescriptor {
    pub de: fn(&mut dyn erased_serde::Deserializer) -> Result<Box<dyn FieldOverride>>,
}

impl<'a, 'de> DeserializeSeed<'de> for &'a FieldOverrideDescriptor {
    type Value = Box<dyn FieldOverride>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        (self.de)(&mut deserializer).map_err(de::Error::custom)
    }
}

pub struct StructOverridesDescriptor {
    fields: HashMap<String, FieldOverrideDescriptor>,
}

impl<'a, 'de> DeserializeSeed<'de> for &'a StructOverridesDescriptor {
    type Value = StructOverrides;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct("StructOverrides", &[], self)
    }
}

impl<'a, 'de> de::Visitor<'de> for &'a StructOverridesDescriptor {
    type Value = StructOverrides;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid `PrefabData` map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut overrides = StructOverrides {
            fields: Default::default(),
        };
        while let Some(key) = map.next_key::<String>()? {
            let field_descriptor = self
                .fields
                .get(&key)
                .ok_or_else(|| de::Error::unknown_field(key.as_str(), &[]))?;

            match overrides.fields.entry(key) {
                Entry::Occupied(occupied) => {
                    return Err(de::Error::custom(format!(
                        "duplicate field `{}`",
                        occupied.key()
                    )));
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(map.next_value_seed(field_descriptor)?);
                }
            }
        }
        Ok(overrides)
    }
}

pub struct StructOverrides {
    fields: HashMap<String, Box<dyn FieldOverride>>,
}

impl MapEntities for StructOverrides {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for (_, v) in &mut self.fields {
            v.map_entities(entity_map)?;
        }
        Ok(())
    }
}

impl FieldOverride for StructOverrides {
    fn apply(&self, target: &mut dyn Any) {
        for (_, v) in &self.fields {
            v.apply(target);
        }
    }

    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        MapEntities::map_entities(self, entity_map)
    }
}

fn register<T: Default + Struct + 'static>() {
    let mut struct_descriptor = StructOverridesDescriptor {
        fields: Default::default(),
    };

    let temp = T::default();
    for (i, field) in temp.iter_fields().enumerate() {
        let name = temp.name_at(i).unwrap();
        let id = field.type_id();

        // let field_descriptor: FieldOverrideDescriptor; // self.find_field(id)
        // struct_descriptor.fields.insert(name.to_string(), field_descriptor);
    }

    let as_field_descriptor = FieldOverrideDescriptor {
        de: |deserializer| Ok(Box::new(struct_descriptor.deserialize(deserializer)?)),
    };
}
