use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::prelude::{PrefabAppBuilder, PrefabData};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Primitives {
    cube: Handle<Mesh>,
}

impl FromWorld for Primitives {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let cube = meshes.add(shape::Cube { size: 1.0 }.into());
        Self { cube }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect)]
pub struct CubePrefab;

impl PrefabData for CubePrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        let mesh = world.get_resource::<Primitives>().unwrap().cube.clone();

        world.entity_mut(root).with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh,
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
