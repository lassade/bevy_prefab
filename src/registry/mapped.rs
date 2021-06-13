use anyhow::Result;
use bevy::ecs::{
    component::Component,
    entity::{EntityMap, MapEntities},
    world::{EntityMut, World},
};

///////////////////////////////////////////////////////////////////////////////

pub type MapWorldComponentsFn = fn(&mut World, &EntityMap) -> Result<()>;

pub type MapEntityComponentsFn = fn(&mut EntityMut, &EntityMap) -> Result<()>;

#[derive(Default)]
pub(crate) struct ComponentEntityMapperRegistry {
    world: Vec<MapWorldComponentsFn>,
    entity: Vec<MapEntityComponentsFn>,
}

impl ComponentEntityMapperRegistry {
    /// Map entities on all registered components in the entire [`World`]
    pub fn map_world_components(&self, world: &mut World, entity_map: &EntityMap) -> Result<()> {
        for map in &self.world {
            (map)(world, &entity_map)?;
        }
        Ok(())
    }

    /// Map entities on all registered components for a single [`Entity`]
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

    /// Register a component that references other entities
    pub fn register<T>(&mut self)
    where
        T: Component + MapEntities + Clone,
    {
        // maps entities all components in the world
        self.world.push(|world, entity_map| {
            let mut query = world.query::<&mut T>();
            for mut component in query.iter_mut(world) {
                component.map_entities(entity_map)?;
            }
            Ok(())
        });

        // maps entities in this component for a single entity
        self.entity.push(|entity, entity_map| {
            if let Some(mut component) = entity.get_mut::<T>() {
                component.map_entities(entity_map)?;
            }
            Ok(())
        });
    }
}
