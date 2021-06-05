use std::{any::TypeId, sync::Arc};

use anyhow::Result;
use bevy::utils::HashMap;
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

    pub fn find_by_type<K: 'static>(&self) -> Option<&T> {
        self.typed
            .get(&TypeId::of::<K>())
            .and_then(|i| self.contents.get(*i))
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
