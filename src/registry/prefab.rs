use std::any::{type_name, TypeId};

use anyhow::Result;
use bevy::prelude::{Entity, World};
use serde::Deserialize;

use crate::{BoxedPrefabData, PrefabData};

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
pub type PrefabDescriptorRegistry = Registry<PrefabDescriptor>;

impl Default for PrefabDescriptorRegistry {
    fn default() -> Self {
        let registry = Self::empty();
        registry
            .register_aliased::<()>("Prefab".to_string())
            .unwrap();
        registry
    }
}

impl PrefabDescriptorRegistry {
    pub fn register<T>(&self) -> Result<()>
    where
        T: PrefabData + Default + Clone + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        self.register_aliased::<T>(shorten_name(type_name::<T>()))
    }

    pub fn register_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: PrefabData + Default + Clone + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let type_info = (TypeId::of::<T>(), type_name::<T>());
        lock.register_internal(alias, type_info, || PrefabDescriptor {
            de: |deserializer| {
                let value: T = Deserialize::deserialize(deserializer)?;
                Ok(BoxedPrefabData(Box::new(value)))
            },
            default: || BoxedPrefabData(Box::new(T::default())),
            construct: |world, root| {
                world
                    .get_entity_mut(root)
                    .and_then(|e| e.get::<T>().cloned())
                    .ok_or_else(|| todo!())
                    .and_then(|data| T::construct(&data, world, root))
            },
        })
    }
}

/// Make [`std::any::type_name`] more human readable by trimming the type path
pub(crate) fn shorten_name(input: &str) -> String {
    let mut chars = input.chars().rev();
    let mut output = String::with_capacity(input.len()); // Reduce the number of allocations
    let mut depth = 0usize;
    let mut k = usize::MAX;
    while let Some(c) = chars.next() {
        if c == '>' {
            output.push('>');
            depth += 1;
        } else if c == '<' {
            output.push('<');
            depth -= 1;
        } else if c == ':' {
            if depth == 0 {
                break;
            }
            chars.next(); // skip next
            k = depth;
        } else if k != depth {
            output.push(c);
        }
    }
    // TODO: Find a better way that doesn't rely on yet another allocation
    output.chars().rev().collect()
}
