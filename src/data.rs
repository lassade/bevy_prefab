use std::fmt::Debug;

use anyhow::Result;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    world::{EntityMut, World},
};
use serde::{Deserialize, Serialize};

///////////////////////////////////////////////////////////////////////////////

pub trait PrefabData: PrefabDataHelper + Debug {
    /// Construct function called once on spawn
    fn construct(&self, world: &mut World, root: Entity) -> Result<()>;
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub(crate) Box<dyn PrefabData + Send + Sync>);

///////////////////////////////////////////////////////////////////////////////

/// Helper default functions
pub trait PrefabDataHelper {
    /// Copies it self in the prefab instance so that self will be available during runtime,
    /// but doesn't override the previously if already has
    fn copy_to_instance(&self, instance: &mut EntityMut);
}

impl<T> PrefabDataHelper for T
where
    T: PrefabData + Clone + Component + 'static,
{
    fn copy_to_instance(&self, entity: &mut EntityMut) {
        if !entity.contains::<T>() {
            entity.insert(self.clone());
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BlankPrefab;

impl PrefabData for BlankPrefab {
    fn construct(&self, _: &mut World, _: Entity) -> Result<()> {
        Ok(())
    }
}
