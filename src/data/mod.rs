use bevy::ecs::entity::{EntityMap, MapEntities, MapEntitiesError};

mod data;
mod overrides;

pub use data::*;
pub use overrides::*;

#[derive(Clone)]
pub struct BoxedPrefabOverrides(pub Box<dyn Override>);

impl MapEntities for BoxedPrefabOverrides {
    #[inline]
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0.map_entities(entity_map)
    }
}
