use std::fmt::Debug;

use anyhow::Result;
use bevy::ecs::world::World;

pub trait PrefabData: Debug {
    fn construct(&self, world: &mut World) -> Result<()>;
}

impl PrefabData for () {
    fn construct(&self, _: &mut World) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub(crate) Box<dyn PrefabData + Send + Sync>);
