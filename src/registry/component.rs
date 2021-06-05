use std::any::{type_name, TypeId};

use anyhow::Result;
use bevy::ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    world::{EntityMut, World},
};
use serde::Deserialize;

use super::Registry;

pub(crate) type ComponentDeserializerFn =
    fn(&mut dyn erased_serde::Deserializer, &mut EntityMut) -> Result<()>;

pub(crate) type ComponentCopyFn = fn(&World, &mut World, Entity, Entity) -> ();

#[derive(Clone)]
pub struct ComponentDescriptor {
    pub(crate) de: ComponentDeserializerFn,
    pub(crate) copy: ComponentCopyFn,
}

pub type ComponentDescriptorRegistry = Registry<ComponentDescriptor>;

impl Default for ComponentDescriptorRegistry {
    #[inline(always)]
    fn default() -> Self {
        Self::empty()
    }
}

impl ComponentDescriptorRegistry {
    pub fn register<T>(&self) -> Result<()>
    where
        T: Component + Clone + for<'de> Deserialize<'de> + 'static,
    {
        self.register_aliased::<T>(shorten_name(type_name::<T>()))
    }

    pub fn register_aliased<T>(&self, alias: String) -> Result<()>
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

    pub fn register_group_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: Bundle + Clone + for<'de> Deserialize<'de> + 'static,
    {
        self.register_inner::<T>(
            alias,
            |deserializer, entity| {
                let value: T = Deserialize::deserialize(deserializer)?;
                entity.insert_bundle(value);
                Ok(())
            },
            |from_world, to_world, from_entity, to_entity| {
                let from = from_world.get::<T>(from_entity).unwrap();
                to_world.entity_mut(to_entity).insert_bundle(from.clone());
            },
        )
    }

    #[inline]
    pub fn register_inner<T>(
        &self,
        alias: String,
        de: ComponentDeserializerFn,
        copy: ComponentCopyFn,
    ) -> Result<()>
    where
        T: for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let type_info = (TypeId::of::<T>(), type_name::<T>());
        lock.register_internal(alias, type_info, || ComponentDescriptor { de, copy })
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
