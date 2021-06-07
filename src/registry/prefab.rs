use std::any::{type_name, TypeId};

use anyhow::Result;
use bevy::prelude::{Entity, World};
use serde::Deserialize;

use crate::{data::BlankPrefab, BoxedPrefabData, PrefabData};

use super::Registry;

pub(crate) type PrefabDeserializerFn =
    fn(&mut dyn erased_serde::Deserializer) -> Result<BoxedPrefabData>;

pub(crate) type PrefabDefaultFn = fn() -> BoxedPrefabData;

pub(crate) type PrefabConstructFn = fn(&mut World, Entity) -> Result<()>;

#[derive(Clone)]
pub struct PrefabDescriptor {
    pub(crate) de: PrefabDeserializerFn,
    pub(crate) default: PrefabDefaultFn,
    pub(crate) construct: PrefabConstructFn,
}

/// Registry of all prefab types available
///
/// **NOTE** The alias `"Prefab"` is registered by default, and uses [`()`] as their [`PrefabData`];
pub(crate) type PrefabDescriptorRegistry = Registry<PrefabDescriptor>;

impl Default for PrefabDescriptorRegistry {
    fn default() -> Self {
        let mut registry = Self::empty();
        registry
            .register_aliased::<BlankPrefab>("Prefab".to_string())
            .unwrap();
        registry
    }
}

impl PrefabDescriptorRegistry {
    pub fn register_aliased<T>(&mut self, alias: String) -> Result<()>
    where
        T: PrefabData + Default + Clone + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        let type_info = (TypeId::of::<T>(), type_name::<T>());
        self.register_internal(alias, type_info, || PrefabDescriptor {
            de: |deserializer| {
                let value: T = Deserialize::deserialize(deserializer)?;
                Ok(BoxedPrefabData(Box::new(value)))
            },
            default: || BoxedPrefabData(Box::new(T::default())),
            construct: |world, root| T::default().construct_instance(world, root),
        })
    }
}
