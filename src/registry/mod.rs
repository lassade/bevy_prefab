use std::{any::TypeId, sync::Arc};

use anyhow::Result;
use bevy::{
    ecs::{
        entity::EntityMap,
        world::{EntityMut, World},
    },
    utils::HashMap,
};
use parking_lot::RwLock;
use thiserror::Error;

mod component;
mod prefab;

///////////////////////////////////////////////////////////////////////////////

pub use component::*;
pub use prefab::*;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("alias `{0}` already registered")]
    AliasAlreadyRegistered(String),
    #[error("type `{0}` already registered")]
    TypeAlreadyRegistered(&'static str),
}

pub(crate) struct RegistryInner<T> {
    contents: Vec<T>,
    named: HashMap<String, usize>,
    typed: HashMap<TypeId, usize>,
}

impl<T> RegistryInner<T> {
    pub fn find_by_name(&self, name: &str) -> Option<&T> {
        self.named.get(name).and_then(|i| self.contents.get(*i))
    }

    pub fn find_by_type(&self, type_id: TypeId) -> Option<&T> {
        self.typed.get(&type_id).and_then(|i| self.contents.get(*i))
    }

    fn register_internal(
        &mut self,
        alias: String,
        type_info: (TypeId, &'static str),
        build: impl Fn() -> T,
    ) -> Result<()> {
        use std::collections::hash_map::Entry::*;

        let (type_id, type_name) = type_info;
        match (self.typed.entry(type_id), self.named.entry(alias)) {
            (_, Occupied(alias)) => Err(RegistryError::AliasAlreadyRegistered(
                alias.key().to_string(),
            ))?,
            (Occupied(_), _) => Err(RegistryError::TypeAlreadyRegistered(type_name))?,
            (Vacant(id), Vacant(alias)) => {
                let i = self.contents.len();
                self.contents.push((build)());
                alias.insert(i);
                id.insert(i);
                Ok(())
            }
        }
    }
}

pub struct Registry<T: Send + Sync> {
    pub(crate) lock: Arc<RwLock<RegistryInner<T>>>,
}

impl<T: Send + Sync> Registry<T> {
    pub(crate) fn empty() -> Self {
        Self {
            lock: Arc::new(RwLock::new(RegistryInner {
                contents: Default::default(),
                named: Default::default(),
                typed: Default::default(),
            })),
        }
    }
}

impl<T: Send + Sync> Clone for Registry<T> {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock.clone(),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub type MapWorldComponentsFn = fn(&mut World, &EntityMap);

pub type MapEntityComponentsFn = fn(&mut EntityMut, &EntityMap);

#[derive(Default)]
pub(crate) struct ComponentEntityMapperRegistryInner {
    world: Vec<MapWorldComponentsFn>,
    entity: Vec<MapEntityComponentsFn>,
}

impl ComponentEntityMapperRegistryInner {
    pub fn map_world_components(&self, world: &mut World, entity_map: &EntityMap) {
        for map in &self.world {
            (map)(world, &entity_map);
        }
    }

    pub fn map_entity_components(&self, entity: &mut EntityMut, entity_map: &EntityMap) {
        for map in &self.entity {
            (map)(entity, &entity_map);
        }
    }
}

pub struct ComponentEntityMapperRegistry {
    pub(crate) lock: Arc<RwLock<ComponentEntityMapperRegistryInner>>,
}

impl ComponentEntityMapperRegistry {
    pub(crate) fn empty() -> Self {
        Self {
            lock: Arc::new(RwLock::new(Default::default())),
        }
    }
}

impl Clone for ComponentEntityMapperRegistry {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock.clone(),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

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
