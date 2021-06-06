use std::sync::Arc;

use anyhow::Result;
use bevy::ecs::{
    component::Component,
    entity::{EntityMap, MapEntities},
    world::{EntityMut, World},
};
use parking_lot::RwLock;
use serde::Deserialize;

///////////////////////////////////////////////////////////////////////////////

pub type MapWorldComponentsFn = fn(&mut World, &EntityMap) -> Result<()>;

pub type MapEntityComponentsFn = fn(&mut EntityMut, &EntityMap) -> Result<()>;

#[derive(Default)]
pub(crate) struct ComponentEntityMapperRegistryInner {
    world: Vec<MapWorldComponentsFn>,
    entity: Vec<MapEntityComponentsFn>,
}

impl ComponentEntityMapperRegistryInner {
    pub fn map_world_components(&self, world: &mut World, entity_map: &EntityMap) -> Result<()> {
        for map in &self.world {
            (map)(world, &entity_map)?;
        }
        Ok(())
    }

    pub fn map_entity_components(
        &self,
        entity: &mut EntityMut,
        entity_map: &EntityMap,
    ) -> Result<()> {
        for map in &self.entity {
            (map)(entity, &entity_map)?;
        }
        Ok(())
    }
}

pub struct ComponentEntityMapperRegistry {
    pub(crate) lock: Arc<RwLock<ComponentEntityMapperRegistryInner>>,
}

impl ComponentEntityMapperRegistry {
    pub fn register<T>(&self)
    where
        T: Component + MapEntities + Clone + for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();

        // maps entities all components in the world
        lock.world.push(|world, entity_map| {
            let mut query = world.query::<&mut T>();
            for mut component in query.iter_mut(world) {
                component.map_entities(entity_map)?;
            }
            Ok(())
        });

        // maps entities in this component for a single entity
        lock.entity.push(|entity, entity_map| {
            if let Some(mut component) = entity.get_mut::<T>() {
                component.map_entities(entity_map)?;
            }
            Ok(())
        });
    }
}

impl Default for ComponentEntityMapperRegistry {
    fn default() -> Self {
        Self {
            lock: Arc::new(RwLock::new(Default::default())),
        }
    }
}

impl Clone for ComponentEntityMapperRegistry {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock.clone(),
        }
    }
}
