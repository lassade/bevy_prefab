use std::fmt::Debug;

use bevy::{
    ecs::world::World,
    math::{Quat, Vec3},
    prelude::Transform,
    reflect::TypeUuid,
};
use serde::{Deserialize, Serialize};

pub mod app;
pub mod builtin;
pub mod command;
pub mod data;
pub mod de;
pub mod loader;
pub mod manager;
pub mod registry;

use crate::data::{BoxedPrefabData, PrefabData};

pub mod prelude {
    pub use crate::app::*;
    pub use crate::command::PrefabCommands;
    pub use crate::data::{BoxedPrefabData, PrefabData};
    pub use crate::Prefab;
}

use crate::registry::PrefabConstructFn;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, TypeUuid)]
#[uuid = "58bc173f-8f5e-4200-88bc-9f12ae9f87cc"]
pub struct Prefab {
    defaults: BoxedPrefabData,
    transform: Transform,
    world: World,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PrefabTransformOverride {
    translation: Option<Vec3>,
    rotation: Option<Quat>,
    scale: Option<Vec3>,
}

///////////////////////////////////////////////////////////////////////////////

/// Tags a prefab with pending instancing
#[derive(Default, Debug, Clone)]
pub struct PrefabNotInstantiatedTag;

/// Tags a prefab as missing
#[derive(Default, Debug, Clone)]
pub struct PrefabMissingTag;

///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
/// Overrides the prefab construct function, needed for procedural prefabs
pub struct PrefabConstruct(PrefabConstructFn);
