use std::any::{type_name, TypeId};

use anyhow::Result;
use bevy::{
    ecs::{
        //bundle::Bundle,
        component::Component,
        entity::Entity,
        world::{EntityMut, World},
    },
    reflect::Uuid,
};
use serde::Deserialize;
use thiserror::Error;

use super::Registry;

pub(crate) type ComponentDeserializerFn =
    fn(&mut dyn erased_serde::Deserializer, &mut EntityMut) -> Result<()>;

pub(crate) type ComponentCopyFn = fn(&World, &mut World, Entity, Entity) -> ();

#[derive(Clone)]
pub struct ComponentDescriptor {
    pub(crate) de: ComponentDeserializerFn,
    pub(crate) copy: ComponentCopyFn,
}

pub(crate) type ComponentDescriptorRegistry = Registry<ComponentDescriptor>;

impl Default for ComponentDescriptorRegistry {
    #[inline(always)]
    fn default() -> Self {
        Self::empty()
    }
}

impl ComponentDescriptorRegistry {
    /// Private component for internal use only
    pub(crate) fn register_private<T>(&mut self, alias: String) -> Result<()>
    where
        T: Component + Clone,
    {
        self.register_inner::<T>(
            alias,
            |deserializer, _| {
                serde::de::IgnoredAny::deserialize(deserializer)?;
                Ok(())
            },
            |from_world, to_world, from_entity, to_entity| {
                let from = from_world.get::<T>(from_entity).unwrap();
                to_world.entity_mut(to_entity).insert(from.clone());
            },
        )
    }

    /// Components that aren't serialized but must also be inserted
    pub fn register_non_serializable<T>(&mut self, alias: String) -> Result<()>
    where
        T: Component + Default + Clone,
    {
        self.register_inner::<T>(
            alias,
            |deserializer, entity| {
                serde::de::IgnoredAny::deserialize(deserializer)?;
                entity.insert(T::default());
                Ok(())
            },
            |from_world, to_world, from_entity, to_entity| {
                let from = from_world.get::<T>(from_entity).unwrap();
                to_world.entity_mut(to_entity).insert(from.clone());
            },
        )
    }

    pub fn register<T>(&mut self, alias: String) -> Result<()>
    where
        T: Component + Clone + for<'de> Deserialize<'de> + 'static,
    {
        self.register_inner::<T>(
            alias,
            |deserializer, entity| {
                let value: T = Deserialize::deserialize(deserializer)?;
                entity.insert(value);
                Ok(())
            },
            |from_world, to_world, from_entity, to_entity| {
                let from = from_world.get::<T>(from_entity).unwrap();
                to_world.entity_mut(to_entity).insert(from.clone());
            },
        )
    }

    /// Prefab data is added component, but shouldn't be inserted as a normal component
    pub(crate) fn register_prefab_data<T>(&mut self, alias: String) -> Result<()>
    where
        T: Component + Clone,
    {
        #[derive(Error, Debug)]
        pub enum PrefabDataComponentRegistryError {
            #[error("prefab data `{0}` can't be added as component")]
            PrefabDataInsertedAsComponent(&'static str),
        }

        self.register_inner::<T>(
            alias,
            |_, _| {
                // prefab data component will always fail to deserialize
                Err(
                    PrefabDataComponentRegistryError::PrefabDataInsertedAsComponent(
                        type_name::<T>(),
                    )
                    .into(),
                )
            },
            |from_world, to_world, from_entity, to_entity| {
                let from = from_world.get::<T>(from_entity).unwrap();
                to_world.entity_mut(to_entity).insert(from.clone());
            },
        )
    }

    #[inline]
    fn register_inner<T>(
        &mut self,
        alias: String,
        de: ComponentDeserializerFn,
        copy: ComponentCopyFn,
    ) -> Result<()>
    where
        T: 'static,
    {
        // Make sure the uuid is unique
        let mut uuid;
        loop {
            uuid = Uuid::new_v4();
            if !self.by_uuid.contains_key(&uuid) {
                break;
            }
        }

        let type_info = (TypeId::of::<T>(), uuid, type_name::<T>());
        self.register_internal(alias, type_info, || ComponentDescriptor { de, copy })?;
        Ok(())
    }
}

// TODO: Save and load between interation and new component
// /// List of all Uuids for each component alias
// pub struct TableOfComponentsUuidByName(Vec<(String, Uuid)>);
