//! Prefab and Scene format for bevy
//!
//! ```ron,ignore
//! Prefab (
//!     defaults: { ... },
//!     scene: [
//!         Entity (
//!             id: 67234,
//!             components: [
//!                 Name(("Root")),
//!                 Transform(( translation: (0, 0, -10) )),
//!                 // Mesh(Embedded(4349)),
//!             ]
//!         ),
//!         Lamp (
//!             id: 95649,
//!             source: External(
//!                 uuid: "76500818-9b39-4655-9d32-8f1ac0ecbb41",
//!                 path: "prefabs/lamp.prefab",
//!             ),
//!             transform: (
//!                 position: (0, 0, 0),
//!                 rotation: (0, 0, 0, 1),
//!                 scale: None,
//!             ),
//!             parent: Some(67234),
//!             data: (
//!                 light_color: LinRgba(1, 0, 0, 1),
//!                 light_strength: 2,
//!             ),
//!         ),
//!     ],
//!     // embedded: [
//!     //     Mesh(4349): { ... }
//!     // ],
//! )
//! ```

use std::fmt::Debug;

use bevy::{
    asset::Handle,
    ecs::{
        entity::{Entity, EntityMap},
        world::World,
    },
    math::{Quat, Vec3},
    prelude::Transform,
    reflect::{TypeUuid, Uuid},
};
use serde::{Deserialize, Serialize};

pub mod data;
pub mod de;
pub mod manager;
pub mod registry;

pub use data::{BoxedPrefabData, PrefabData};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct PrefabInstance {
    id: Entity,
    source: Handle<Prefab>,
    // Overrides
    parent: Option<Entity>,
    transform: PrefabInstanceTransform,
    // Data feed to construct script
    data: BoxedPrefabData,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PrefabInstanceTransform {
    translation: Option<Vec3>,
    rotation: Option<Quat>,
    scale: Option<Vec3>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrefabVariantId {
    uuid: Uuid,
    name: String,
}

impl Default for PrefabVariantId {
    fn default() -> Self {
        Self {
            uuid: Uuid::default(),
            name: "Prefab".to_string(),
        }
    }
}

#[derive(Debug, TypeUuid)]
#[uuid = "58bc173f-8f5e-4200-88bc-9f12ae9f87cc"]
pub struct Prefab {
    defaults: BoxedPrefabData,
    transform: Transform,
    world: World,
    nested_prefabs: Vec<PrefabInstance>,
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//         assert_eq!(2 + 2, 4);
//     }
// }
