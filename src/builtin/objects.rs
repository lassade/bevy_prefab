use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::prelude::{PrefabAppBuilder, PrefabData};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct StaticMeshPrefab {
    mesh: Handle<Mesh>,
    material: Option<Handle<StandardMaterial>>,
}

impl PrefabData for StaticMeshPrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        let mesh = self.mesh.clone();
        let material = self.material.clone().unwrap_or_default();

        world.entity_mut(root).with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh,
                material,
                ..Default::default()
            });
        });

        Ok(())
    }
}

// TODO: PointLightPrefab
// TODO: DirectionalLightPrefab
// TODO: PerspectiveCameraPrefab
// TODO: OrthographicCameraPrefab

///////////////////////////////////////////////////////////////////////////////

pub fn register_objects_prefabs(app_builder: &mut AppBuilder) {
    app_builder.register_prefab::<StaticMeshPrefab>();
}
