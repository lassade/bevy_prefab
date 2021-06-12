use std::fmt::Debug;

use anyhow::Result;
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        world::{EntityMut, World},
    },
    reflect::{TypeUuid, Uuid, Reflect},
};
use serde::{Deserialize, Serialize};

///////////////////////////////////////////////////////////////////////////////

pub trait PrefabData: PrefabDataHelper + Debug + Send + Sync + 'static {
    /// Construct function called once on spawn
    fn construct(&self, world: &mut World, root: Entity) -> Result<()>;
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub(crate) Box<dyn PrefabData>);

///////////////////////////////////////////////////////////////////////////////

/// Helper default functions
pub trait PrefabDataHelper {
    /// Copies it self in the prefab instance so that self will be available during runtime,
    /// but doesn't override the previously if already has
    fn copy_to_instance(&self, instance: &mut EntityMut);

    /// Constructs prefabs using the instance data or default to this data
    fn construct_instance(&self, world: &mut World, root: Entity) -> Result<()>;

    /// Uuid from [`TypeUuid`]
    fn type_uuid(&self) -> Uuid;
}

impl<T> PrefabDataHelper for T
where
    T: PrefabData + TypeUuid + Clone + Component,
{
    fn copy_to_instance(&self, entity: &mut EntityMut) {
        if !entity.contains::<T>() {
            entity.insert(self.clone());
        }
    }

    fn construct_instance(&self, world: &mut World, root: Entity) -> Result<()> {
        // TODO: quite bit of cloning is required, maybe there's a better ways but I digress
        let mut entity = world.entity_mut(root);
        if let Some(data) = entity.get::<T>() {
            // use the prefab component data to run the construct function
            data.clone().construct(world, root)
        } else {
            // insert missing prefab data component
            entity.insert(self.clone());
            // run the construct function using the original copy of the data,
            // this data could be `Default::default` or the data from the source prefab
            self.construct(world, root)
        }
    }

    fn type_uuid(&self) -> Uuid {
        T::TYPE_UUID
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, TypeUuid, Reflect)]
#[uuid = "3c603f24-9a89-45c3-8f4a-087a28f006df"]
pub struct BlankPrefab;

impl PrefabData for BlankPrefab {
    fn construct(&self, _: &mut World, _: Entity) -> Result<()> {
        Ok(())
    }
}
