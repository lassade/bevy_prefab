use std::any::{type_name, TypeId};

use anyhow::Result;
use bevy::{
    prelude::{Entity, World},
    reflect::{TypeUuid, Uuid},
};
use serde::Deserialize;

use crate::{data::BlankPrefab, BoxedPrefabData, PrefabData};

use super::Registry;

pub(crate) type PrefabDeserializerFn =
    fn(&mut dyn erased_serde::Deserializer) -> Result<BoxedPrefabData>;

pub(crate) type PrefabDefaultFn = fn() -> BoxedPrefabData;

pub(crate) type PrefabConstructFn = fn(&mut World, Entity) -> Result<()>;

pub(crate) type PrefabUuidFn = fn() -> Uuid;

#[derive(Clone)]
pub struct PrefabDescriptor {
    pub(crate) source_prefab_required: bool,
    pub(crate) de: PrefabDeserializerFn,
    pub(crate) default: PrefabDefaultFn,
    pub(crate) construct: PrefabConstructFn,
    pub(crate) uuid: PrefabUuidFn,
}

/// Registry of all prefab types available
///
/// **NOTE** The alias `"Prefab"` is registered by default, and uses [`()`] as their [`PrefabData`];
pub(crate) type PrefabDescriptorRegistry = Registry<PrefabDescriptor>;

impl Default for PrefabDescriptorRegistry {
    fn default() -> Self {
        let mut registry = Self::empty();
        registry
            .register_aliased::<BlankPrefab>("Prefab".to_string(), true)
            .unwrap();
        registry
    }
}

impl PrefabDescriptorRegistry {
    pub fn register_aliased<T>(&mut self, alias: String, source_prefab_required: bool) -> Result<()>
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
        let type_info = (TypeId::of::<T>(), T::TYPE_UUID, type_name::<T>());
        self.register_internal(alias, type_info, || PrefabDescriptor {
            source_prefab_required,
            de: |deserializer| {
                let value: T = Deserialize::deserialize(deserializer)?;
                Ok(BoxedPrefabData(Box::new(value)))
            },
            default: || BoxedPrefabData(Box::new(T::default())),
            construct: |world, root| T::default().construct_instance(world, root),
            uuid: || T::TYPE_UUID,
        })?;
        Ok(())
    }
}
