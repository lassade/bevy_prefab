use std::{any::type_name, collections::hash_map::Entry};

use anyhow::Result;
use serde::Deserialize;

use crate::{BoxedPrefabData, PrefabData};

use super::{Registry, RegistryError};

#[derive(Clone)]
pub struct PrefabDescriptor {
    pub(crate) de: &'static dyn Fn(&mut dyn erased_serde::Deserializer) -> Result<BoxedPrefabData>,
}

pub type PrefabDescriptorRegistry = Registry<PrefabDescriptor>;

impl PrefabDescriptorRegistry {
    pub fn register<T>(&self) -> Result<()>
    where
        T: PrefabData + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        self.register_aliased::<T>(shorten_name(type_name::<T>()))
    }

    pub fn register_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: PrefabData + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let entry = lock.named.entry(alias);
        match entry {
            Entry::Occupied(occupied) => Err(RegistryError::AliasAlreadyRegistered(
                occupied.key().to_string(),
            ))?,
            Entry::Vacant(vacant) => {
                vacant.insert(PrefabDescriptor {
                    de: &|deserializer| {
                        let value: T = Deserialize::deserialize(deserializer)?;
                        Ok(BoxedPrefabData(Box::new(value)))
                    },
                });
                Ok(())
            }
        }
    }
}

/// Make [`std::any::type_name`] more human readable by trimming the type path
pub(crate) fn shorten_name(input: &str) -> String {
    let mut chars = input.chars().rev();
    let mut output = String::new();
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
