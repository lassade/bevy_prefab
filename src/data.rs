use std::fmt::Debug;

use anyhow::Result;
use bevy::ecs::{
    entity::Entity,
    world::{EntityMut, World},
};

pub trait PrefabData: Debug {
    fn construct(&self, world: &mut World, root: Entity) -> Result<()>;

    /// Copies it self in the prefab instance so that self will be available during runtime
    fn copy_to_instance(&self, instance: &mut EntityMut);
}

impl PrefabData for () {
    fn construct(&self, _: &mut World, _: Entity) -> Result<()> {
        Ok(())
    }

    fn copy_to_instance(&self, _: &mut EntityMut) {}
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub(crate) Box<dyn PrefabData + Send + Sync>);
