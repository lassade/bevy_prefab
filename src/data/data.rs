use std::fmt::Debug;

use anyhow::Result;
use bevy::{ecs::{component::Component, entity::{Entity, EntityMap}, world::World}, reflect::{Reflect, TypeUuid, Uuid}};
use serde::{Deserialize, Serialize};

use super::BoxedPrefabOverrides;

///////////////////////////////////////////////////////////////////////////////

pub trait PrefabData: PrefabDataHelper + Debug + Send + Sync + 'static {
    /// Construct function called once on spawn
    fn construct(&self, world: &mut World, root: Entity) -> Result<()>;
    
    /// Find entities references
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<()> {
        let _ = entity_map;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BoxedPrefabData(pub Box<dyn PrefabData>);

///////////////////////////////////////////////////////////////////////////////

/// Helper default functions
pub trait PrefabDataHelper {
    /// Constructs prefabs instances using the instance data or using self as a default
    /// is also responsible to apply any prefab overrides
    fn apply_overrides_and_construct_instance(&self, world: &mut World, root: Entity, prefab_to_instance: &EntityMap) -> Result<()>;

    /// Uuid from [`TypeUuid`]
    fn type_uuid(&self) -> Uuid;
}

impl<T> PrefabDataHelper for T
where
    T: PrefabData + TypeUuid + Reflect + Clone + Component,
{
    fn apply_overrides_and_construct_instance(&self, world: &mut World, root: Entity, prefab_to_instance: &EntityMap) -> Result<()> {
        // TODO: quite bit of cloning is required, maybe there's a better ways but I digress
        let mut entity = world.entity_mut(root);

        let overrides = entity
            .get::<BoxedPrefabOverrides>()
            .map(|overrides| 
                // SAFETY used to apply overrides in the prefab data,
                // no changes will be made in the entity archetype so no data will be invalidated
                unsafe { &*(overrides as *const BoxedPrefabOverrides) }
            );
        
        // create defaults
        let mut data = self.clone();

        // map data entities to the instance space
        data.map_entities(prefab_to_instance)?;

        if let Some(overrides) = overrides {
            // apply overrides
            overrides.0.apply_override(&mut data);

            // insert missing prefab data component
            entity.insert(data.clone());

            // run the prefab construct function using it's data
            data.construct(world, root)
        } else {
            // fast code path since no overrides where added less data cloning is required
            entity.insert(data);

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
