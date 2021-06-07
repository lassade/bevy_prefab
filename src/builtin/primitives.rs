use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::prelude::{PrefabAppBuilder, PrefabData};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Primitives {
    default_material: Handle<StandardMaterial>,
    cube: Handle<Mesh>,
}

impl FromWorld for Primitives {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let cube = meshes.add(shape::Cube { size: 1.0 }.into());

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let default_material = materials.add(Color::GRAY.into());

        Self {
            cube,
            default_material,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect)]
pub struct CubePrefab;

impl PrefabData for CubePrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        let primitives = world.get_resource::<Primitives>().unwrap();
        let mesh = primitives.cube.clone();
        let material = primitives.default_material.clone();

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

// TODO: UVSpherePrefab
// TODO: PlanePrefab
// TODO: CapsulePrefab
// TODO: CylinderPrefab
// TODO: TorusPrefab

///////////////////////////////////////////////////////////////////////////////

pub fn register_primitives_prefabs(app_builder: &mut AppBuilder) {
    let primitives = Primitives::from_world(&mut app_builder.app.world);
    app_builder
        .insert_resource(primitives)
        .register_prefab::<CubePrefab>();
}
