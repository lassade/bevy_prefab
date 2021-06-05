use std::{any::type_name, collections::hash_map::Entry};

use anyhow::Result;
use bevy::ecs::{bundle::Bundle, component::Component, world::EntityMut};
use serde::Deserialize;

use super::{Registry, RegistryError};

pub(crate) type ComponentDeserializerFn =
    dyn Fn(&mut dyn erased_serde::Deserializer, &mut EntityMut) -> Result<()>;

#[derive(Clone)]
pub struct ComponentDescriptor {
    pub(crate) de: &'static ComponentDeserializerFn,
}

pub type ComponentDescriptorRegistry = Registry<ComponentDescriptor>;

impl ComponentDescriptorRegistry {
    pub fn register<T>(&self) -> Result<()>
    where
        T: Component + for<'de> Deserialize<'de> + 'static,
    {
        self.register_aliased::<T>(shorten_name(type_name::<T>()))
    }

    pub fn register_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: Component + for<'de> Deserialize<'de> + 'static,
    {
        self.register_inner::<T>(alias, &|deserializer, entity| {
            let value: T = Deserialize::deserialize(deserializer)?;
            entity.insert(value);
            Ok(())
        })
    }

    pub fn register_group_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: Bundle + for<'de> Deserialize<'de> + 'static,
    {
        self.register_inner::<T>(alias, &|deserializer, entity| {
            let value: T = Deserialize::deserialize(deserializer)?;
            entity.insert_bundle(value);
            Ok(())
        })
    }

    #[inline]
    pub fn register_inner<T>(
        &self,
        alias: String,
        de: &'static ComponentDeserializerFn,
    ) -> Result<()>
    where
        T: for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let entry = lock.named.entry(alias);
        match entry {
            Entry::Occupied(occupied) => Err(RegistryError::AliasAlreadyRegistered(
                occupied.key().to_string(),
            ))?,
            Entry::Vacant(vacant) => {
                vacant.insert(ComponentDescriptor { de });
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
