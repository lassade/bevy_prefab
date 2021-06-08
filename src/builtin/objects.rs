use bevy::{prelude::*, reflect::TypeUuid};
use serde::{Deserialize, Serialize};

use crate::prelude::{PrefabAppBuilder, PrefabData};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[uuid = "9a8c902c-f4d8-4820-b2f5-705122f67af4"]
pub struct StaticMeshPrefab {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl PrefabData for StaticMeshPrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        let mesh = self.mesh.clone();
        let material = self.material.clone();

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
    app_builder.register_prefab::<StaticMeshPrefab>(false);
}
