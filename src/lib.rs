use std::any::Any;

// use anyhow::Result;
// use bevy::{
//     asset::Handle,
//     ecs::world::World,
//     math::{Quat, Vec3},
//     reflect::{TypeUuid, Uuid},
//     transform::components::Parent,
// };

mod de;

///////////////////////////////////////////////////////////////////////////////

// #[derive(Debug)]
// pub struct NodeId(u32);

// #[derive(Debug)]
// pub struct PrefabDescriptor {
//     pub uuid: Uuid,
//     pub name: String,
// }

// #[derive(Debug)]
// pub struct PrefabTransform {
//     pub translation: Vec3,
//     pub rotation: Quat,
//     pub scale: Option<Vec3>,
// }

// #[derive(Debug)]
// pub enum Node {
//     Entity {
//         id: NodeId,
//         data: Vec<()>, // TODO:
//     },
//     Prefab {
//         id: NodeId,
//         source: Handle<Prefab>,
//         // Overrides
//         parent: Option<Parent>,
//         transform: PrefabTransform,
//         // Data feed to construct script
//         data: Option<Box<dyn Any + Send + Sync>>, // TODO:
//     },
// }

// #[derive(Debug, TypeUuid)]
// #[uuid = "58bc173f-8f5e-4200-88bc-9f12ae9f87cc"]
// pub struct Prefab {
//     pub variant: PrefabDescriptor,
//     pub scene: Vec<Node>,
// }

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
