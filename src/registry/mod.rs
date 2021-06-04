use std::sync::Arc;

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
}

pub(crate) struct RegistryInner<T> {
    pub(crate) named: HashMap<String, T>,
}

pub struct Registry<T> {
    pub(crate) lock: Arc<RwLock<RegistryInner<T>>>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            lock: Arc::new(RwLock::new(RegistryInner {
                named: Default::default(),
            })),
        }
    }
}

impl<T> Clone for Registry<T> {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock.clone(),
        }
    }
}
