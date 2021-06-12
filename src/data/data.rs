use std::fmt::Debug;

use anyhow::Result;
use bevy::{
    ecs::{component::Component, entity::Entity, world::World},
    reflect::{Reflect, TypeUuid, Uuid},
};
use serde::{Deserialize, Serialize};

use super::BoxedPrefabOverrides;

///////////////////////////////////////////////////////////////////////////////

pub trait PrefabData: PrefabDataHelper + Debug + Send + Sync + 'static {
    /// Construct function called once on spawn
    fn construct(&self, world: &mut World, root: Entity) -> Result<()>;
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub Box<dyn PrefabData>);

///////////////////////////////////////////////////////////////////////////////

/// Helper default functions
pub trait PrefabDataHelper {
    /// Constructs prefabs instances using the instance data or using self as a default
    /// is also responsible to apply any prefab overrides
    fn construct_instance(&self, world: &mut World, root: Entity) -> Result<()>;

    /// Uuid from [`TypeUuid`]
    fn type_uuid(&self) -> Uuid;
}

impl<T> PrefabDataHelper for T
where
    T: PrefabData + TypeUuid + Reflect + Clone + Component,
{
    fn construct_instance(&self, world: &mut World, root: Entity) -> Result<()> {
        // TODO: quite bit of cloning is required, maybe there's a better ways but I digress
        let mut entity = world.entity_mut(root);

        let overrides = entity
            .get::<BoxedPrefabOverrides>()
            .map(|overrides| 
                // SAFETY used to apply overrides in the prefab data,
                // no changes will be made in the entity archetype so no data will be invalidated
                unsafe { &*(overrides as *const BoxedPrefabOverrides) }
            );

        if let Some(mut data) = entity.get_mut::<T>() {
            // apply overrides
            if let Some(overrides) = overrides {
                overrides.0.apply_override(&mut *data);
            }
            
            // use the prefab component data to run the construct function
            data.clone().construct(world, root)
        } else {
            // create defaults
            let mut data = self.clone();
            if let Some(overrides) = overrides {
                // apply overrides
                overrides.0.apply_override(&mut data);

                // insert missing prefab data component
                entity.insert(data.clone());
    
                // run the prefab construct function using it's data
                data.construct(world, root)
            } else {
                // fast code path since no overrides where necessary less data cloning is required
                entity.insert(data);
    
                // run the construct function using the original copy of the data,
                // this data could be `Default::default` or the data from the source prefab
                self.construct(world, root)
            }
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
