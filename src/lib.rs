//! Prefab and Scene format for bevy
//!
//! ```ron,ignore
//! Prefab {
//!     // None specify the simplest prefab with no construct function
//!     variant: Some({
//!         uuid: <uuid>,
//!         name: <prefab variant name>,
//!     }),
//!     scene: [
//!         Entity {
//!             id: 67234,
//!             components: [
//!                 Name(("Root")),
//!                 Transform({ translation: (0, 0, -10) }),
//!             ]
//!         },
//!         Prefab {
//!             id: 95649,
//!             source: {
//!                 uuid: "76500818-9b39-4655-9d32-8f1ac0ecbb41",
//!                 path: "prefabs/lamp.prefab",
//!             },
//!             transform: {
//!                 position: (0, 0, 0),
//!                 rotation: (0, 0, 0, 1),
//!                 scale: None,
//!             },
//!             parent: Some(67234),
//!             data: {
//!                 light_color: LinRgba(1, 0, 0, 1),
//!                 light_strength: 2,
//!             },
//!         },
//!     ]
//! }
//! ```

use std::any::Any;

use bevy::{
    asset::Handle,
    ecs::{entity::Entity, world::World},
    math::{Quat, Vec3},
    reflect::{TypeUuid, Uuid},
    transform::components::Parent,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

pub mod de;
pub mod registry;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct PrefabNodeId(u32);

#[derive(Debug)]
pub struct PrefabInstance {
    id: PrefabNodeId,
    source: Handle<Prefab>,
    // Overrides
    parent: Option<Parent>,
    transform: PrefabInstanceTransform,
    // Data feed to construct script
    data: Option<BoxedPrefabData>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PrefabInstanceTransform {
    translation: Vec3,
    rotation: Quat,
    scale: Option<Vec3>,
}

#[derive(Debug)]
pub struct BoxedPrefabData(Box<dyn Any + Send + Sync>);

#[derive(Debug)]
pub struct PrefabVariantId {
    uuid: Uuid,
    name: String,
}

#[derive(Debug, TypeUuid)]
#[uuid = "58bc173f-8f5e-4200-88bc-9f12ae9f87cc"]
pub struct Prefab {
    entity_map: HashMap<PrefabNodeId, Entity>,
    world: World,
    nested_prefabs: Vec<PrefabInstance>,
}

// pub trait PrefabData: TypeUuid {
//     fn construct(&self, world: &mut World) -> Result<()>;
// }

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//         assert_eq!(2 + 2, 4);
//     }
// }
