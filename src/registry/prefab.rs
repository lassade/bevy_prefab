use std::any::{type_name, TypeId};

use anyhow::Result;
use bevy::{
    prelude::{Entity, World},
    reflect::{Struct, TypeUuid, Uuid},
};
use serde::Deserialize;

use crate::{
    data::{BlankPrefab, OverrideDescriptor, OverrideRegistry},
    BoxedPrefabData, PrefabData,
};

use super::Registry;

pub(crate) type PrefabDeserializerFn =
    fn(&mut dyn erased_serde::Deserializer) -> Result<BoxedPrefabData>;

pub(crate) type PrefabDefaultFn = fn() -> BoxedPrefabData;

pub(crate) type PrefabConstructFn = fn(&mut World, Entity) -> Result<()>;

#[derive(Clone)]
pub struct PrefabDescriptor {
    pub(crate) source_prefab_required: bool,
    pub(crate) de: PrefabDeserializerFn,
    pub(crate) overrides: OverrideDescriptor,
    pub(crate) default: PrefabDefaultFn,
    pub(crate) construct: PrefabConstructFn,
    pub(crate) uuid: Uuid,
}

/// Registry of all prefab types available
///
/// **NOTE** The alias `"Prefab"` is registered by default, and uses [`()`] as their [`PrefabData`];
pub(crate) struct PrefabDescriptorRegistry {
    pub overrides: OverrideRegistry,
    base: Registry<PrefabDescriptor>,
}

impl Default for PrefabDescriptorRegistry {
    fn default() -> Self {
        let mut registry = Self {
            overrides: Default::default(),
            base: Registry::<PrefabDescriptor>::empty(),
        };
        registry
            .register_aliased::<BlankPrefab>("Prefab".to_string(), true)
            .unwrap();
        registry
    }
}

impl PrefabDescriptorRegistry {
    #[inline]
    pub fn find_by_name(&self, name: &str) -> Option<&PrefabDescriptor> {
        self.base.find_by_name(name)
    }

    // TODO: `source_prefab_required` should be configured statically in a trait not during registration
    pub fn register_aliased<T>(&mut self, alias: String, source_prefab_required: bool) -> Result<()>
    where
        T: PrefabData + TypeUuid + Default + Struct + Clone + for<'de> Deserialize<'de>,
    {
        let PrefabDescriptorRegistry { overrides, base } = self;

        let type_info = (TypeId::of::<T>(), T::TYPE_UUID, type_name::<T>());
        base.register_internal(alias, type_info, || {
            overrides.register_struct::<T>();
            PrefabDescriptor {
                source_prefab_required,
                de: |deserializer| {
                    let value: T = Deserialize::deserialize(deserializer)?;
                    Ok(BoxedPrefabData(Box::new(value)))
                },
                overrides: overrides.find::<T>().unwrap().clone(),
                default: || BoxedPrefabData(Box::new(T::default())),
                construct: |world, root| T::default().construct_instance(world, root),
                uuid: T::TYPE_UUID,
            }
        })?;
        Ok(())
    }
}

// pub(crate) fn prefab_construct<T: PrefabData + Default + Struct + Clone >(
//     world: &mut World,
//     root_entity: Entity,
// ) -> Result<()> {

//     let value = world.entity(root_entity);
//     let T = .get::<T>().cloned();

//     // Take into account the overrides
//     T::default().construct_instance(world, root_entity)
// }
