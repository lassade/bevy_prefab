use std::any::TypeId;

use anyhow::Result;
use bevy::utils::HashMap;
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
}

pub(crate) struct Registry<T> {
    contents: Vec<T>,
    named: HashMap<String, usize>,
    typed: HashMap<TypeId, usize>,
}

impl<T> Registry<T> {
    fn empty() -> Self {
        Self {
            contents: Default::default(),
            named: Default::default(),
            typed: Default::default(),
        }
    }

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
