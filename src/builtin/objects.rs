use bevy::{prelude::*, reflect::TypeUuid};
use serde::{Deserialize, Serialize};

use crate::prelude::{PrefabAppBuilder, PrefabData};

use super::PbrPrimitiveBundle;

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

        world
            .entity_mut(root)
            .insert_bundle(PbrPrimitiveBundle::default())
            .insert_bundle((mesh, material));

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[uuid = "c19276df-0609-4171-a71d-30ef513a92d1"]
pub struct PointLightPrefab {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
}

impl Default for PointLightPrefab {
    fn default() -> Self {
        PointLightPrefab {
            color: Color::new(1.0, 1.0, 1.0),
            intensity: 200.0,
            range: 20.0,
            radius: 0.0,
        }
    }
}

impl PrefabData for PointLightPrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        world.entity_mut(root).insert_bundle(PointLightBundle {
            point_light: PointLight {
                color: self.color,
                intensity: self.intensity,
                range: self.range,
                radius: self.radius,
            },
            ..Default::default()
        });

        Ok(())
    }
}

// TODO: DirectionalLightPrefab
// TODO: PerspectiveCameraPrefab
// TODO: OrthographicCameraPrefab

///////////////////////////////////////////////////////////////////////////////

pub fn register_objects_prefabs(app_builder: &mut AppBuilder) {
    app_builder.register_prefab::<StaticMeshPrefab>(false);
    app_builder.register_prefab::<PointLightPrefab>(false);
}
