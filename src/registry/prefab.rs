use std::{
    any::{type_name, TypeId},
    collections::hash_map::Entry,
};

use anyhow::Result;
use bevy::{
    prelude::{Entity, World},
    reflect::{TypeUuid, Uuid},
    utils::HashMap,
};
use serde::Deserialize;
use thiserror::Error;

use crate::{data::BlankPrefab, BoxedPrefabData, PrefabData};

use super::Registry;

pub(crate) type PrefabDeserializerFn =
    fn(&mut dyn erased_serde::Deserializer) -> Result<BoxedPrefabData>;

pub(crate) type PrefabDefaultFn = fn() -> BoxedPrefabData;

pub(crate) type PrefabConstructFn = fn(&mut World, Entity) -> Result<()>;

#[derive(Clone)]
pub struct PrefabDescriptor {
    pub(crate) uuid: Uuid,
    pub(crate) de: PrefabDeserializerFn,
    pub(crate) default: PrefabDefaultFn,
    pub(crate) construct: PrefabConstructFn,
}

#[derive(Error, Debug)]
enum PrefabRegistryError {
    #[error("type `{0}` have a conflicting uuid")]
    UuidAlreadyRegistered(&'static str),
}

/// Registry of all prefab types available
///
/// **NOTE** The alias `"Prefab"` is registered by default, and uses [`()`] as their [`PrefabData`];
pub(crate) struct PrefabDescriptorRegistry {
    registry: Registry<PrefabDescriptor>,
    tagged: HashMap<Uuid, usize>,
}

impl Default for PrefabDescriptorRegistry {
    fn default() -> Self {
        let mut registry = Self {
            registry: Registry::empty(),
            tagged: Default::default(),
        };

        registry
            .register_aliased::<BlankPrefab>("Prefab".to_string())
            .unwrap();

        registry
    }
}

impl PrefabDescriptorRegistry {
    pub fn find_by_uuid(&self, uuid: &Uuid) -> Option<&PrefabDescriptor> {
        self.tagged
            .get(uuid)
            .and_then(|i| self.registry.contents.get(*i))
    }

    #[inline]
    pub fn find_by_name(&self, name: &str) -> Option<&PrefabDescriptor> {
        self.registry.find_by_name(name)
    }

    #[inline]
    pub fn find_by_type(&self, type_id: TypeId) -> Option<&PrefabDescriptor> {
        self.registry.find_by_type(type_id)
    }

    pub fn register_aliased<T>(&mut self, alias: String) -> Result<()>
    where
        T: PrefabData
            + TypeUuid
            + Default
            + Clone
            + Send
            + Sync
            + for<'de> Deserialize<'de>
            + 'static,
    {
        let type_info = (TypeId::of::<T>(), type_name::<T>());

        match self.tagged.entry(T::TYPE_UUID) {
            Entry::Occupied(_) => {
                Err(PrefabRegistryError::UuidAlreadyRegistered(type_name::<T>()))?
            }
            Entry::Vacant(vacant) => {
                let index =
                    self.registry
                        .register_internal(alias, type_info, || PrefabDescriptor {
                            uuid: T::TYPE_UUID,
                            de: |deserializer| {
                                let value: T = Deserialize::deserialize(deserializer)?;
                                Ok(BoxedPrefabData(Box::new(value)))
                            },
                            default: || BoxedPrefabData(Box::new(T::default())),
                            construct: |world, root| T::default().construct_instance(world, root),
                        })?;

                vacant.insert(index);
                Ok(())
            }
        }
    }
}
