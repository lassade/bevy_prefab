use std::any::TypeId;

use bevy::{reflect::Uuid, utils::HashMap};
use thiserror::Error;

mod component;
mod mapped;
mod prefab;

///////////////////////////////////////////////////////////////////////////////

pub use component::*;
pub use mapped::*;
pub use prefab::*;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("alias `{0}` already registered")]
    AliasAlreadyRegistered(String),
    #[error("type `{0}` already registered")]
    TypeAlreadyRegistered(&'static str),
    #[error("uuid `{0}` already registered")]
    UuidAlreadyRegistered(Uuid),
}

pub(crate) struct Registry<T> {
    reg: Vec<T>,
    by_name: HashMap<String, usize>,
    by_type: HashMap<TypeId, usize>,
    by_uuid: HashMap<Uuid, usize>,
}

impl<T> Registry<T> {
    fn empty() -> Self {
        Self {
            reg: Default::default(),
            by_name: Default::default(),
            by_type: Default::default(),
            by_uuid: Default::default(),
        }
    }

    // TODO: Used to support prefabs uuid deserialization
    // pub fn find_by_uuid(&self, uuid: &Uuid) -> Option<&T> {
    //     self.by_uuid.get(uuid).and_then(|i| self.reg.get(*i))
    // }

    pub fn find_by_name(&self, name: &str) -> Option<&T> {
        self.by_name.get(name).and_then(|i| self.reg.get(*i))
    }

    pub fn find_by_type(&self, type_id: TypeId) -> Option<&T> {
        self.by_type.get(&type_id).and_then(|i| self.reg.get(*i))
    }

    fn register_internal(
        &mut self,
        alias: String,
        type_info: (TypeId, Uuid, &'static str),
        build: impl Fn() -> T,
    ) -> Result<usize, RegistryError> {
        use std::collections::hash_map::Entry::*;

        let (type_id, type_uuid, type_name) = type_info;
        match (
            self.by_type.entry(type_id),
            self.by_name.entry(alias),
            self.by_uuid.entry(type_uuid),
        ) {
            (_, Occupied(alias), _) => Err(RegistryError::AliasAlreadyRegistered(
                alias.key().to_string(),
            ))?,
            (Occupied(_), _, _) => Err(RegistryError::TypeAlreadyRegistered(type_name))?,
            (_, _, Occupied(uuid)) => Err(RegistryError::UuidAlreadyRegistered(*uuid.key()))?,
            (Vacant(id), Vacant(alias), Vacant(uuid)) => {
                let i = self.reg.len();
                self.reg.push((build)());
                alias.insert(i);
                id.insert(i);
                uuid.insert(i);
                Ok(i)
            }
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
