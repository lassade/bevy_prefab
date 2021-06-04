use std::{any::type_name, collections::hash_map::Entry, sync::Arc};

use anyhow::Result;
use bevy::{
    ecs::{bundle::Bundle, component::Component, world::EntityMut},
    utils::HashMap,
};
use parking_lot::RwLock;
use serde::Deserialize;
use thiserror::Error;

#[derive(Clone)]
pub struct ComponentDescriptor {
    pub(crate) de:
        &'static dyn Fn(&mut dyn erased_serde::Deserializer, &mut EntityMut) -> Result<()>,
    //fields: &'static [&'static str],
}

#[derive(Default)]
pub(crate) struct ComponentDescriptorRegistryInner {
    pub(crate) named: HashMap<String, ComponentDescriptor>,
}

#[derive(Clone)]
pub struct ComponentDescriptorRegistry {
    pub(crate) lock: Arc<RwLock<ComponentDescriptorRegistryInner>>,
}

impl Default for ComponentDescriptorRegistry {
    fn default() -> Self {
        Self {
            lock: Arc::new(RwLock::new(Default::default())),
        }
    }
}

impl ComponentDescriptorRegistry {
    pub fn registry<T>(&self) -> Result<()>
    where
        T: Component + for<'de> Deserialize<'de> + 'static,
    {
        self.register_aliased::<T>(shorten_name(type_name::<T>()))
    }

    pub fn register_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: Component + for<'de> Deserialize<'de> + 'static,
    {
        self.register_group_aliased::<(T,)>(alias)
    }

    pub fn register_group_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: Bundle + for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let entry = lock.named.entry(alias);
        match entry {
            Entry::Occupied(occupied) => {
                Err(ComponentDescriptorRegistryError::AliasAlreadyRegistered(
                    occupied.key().to_string(),
                ))?
            }
            Entry::Vacant(vacant) => {
                vacant.insert(ComponentDescriptor {
                    de: &|deserializer, entity| {
                        let value: T = Deserialize::deserialize(deserializer)?;
                        entity.insert_bundle(value);
                        Ok(())
                    },
                });
                Ok(())
            }
        }
    }
}
#[derive(Error, Debug)]
enum ComponentDescriptorRegistryError {
    #[error("alias `{0}` already registered")]
    AliasAlreadyRegistered(String),
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
