use bevy::{prelude::*, reflect::TypeUuid};
use serde::{Deserialize, Serialize};

use crate::prelude::{PrefabAppBuilder, PrefabData};

use super::PbrPrimitiveBundle;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Primitives {
    default_material: Handle<StandardMaterial>,
    cube: Handle<Mesh>,
    uv_sphere: Handle<Mesh>,
    plane: Handle<Mesh>,
    capsule: Handle<Mesh>,
    //cylinder: Handle<Mesh>,
    //torus: Handle<Mesh>,
}

impl FromWorld for Primitives {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let cube = meshes.add(shape::Cube::default().into());
        let uv_sphere = meshes.add(
            shape::UVSphere {
                radius: 0.5,
                ..Default::default()
            }
            .into(),
        );
        let plane = meshes.add(shape::Plane::default().into());
        let capsule = meshes.add(shape::Capsule::default().into());

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let default_material = materials.add(Color::GRAY.into());

        Self {
            default_material,
            cube,
            uv_sphere,
            plane,
            capsule,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[inline]
fn common_construct(
    world: &mut World,
    root: Entity,
    shape: impl Fn(&Primitives) -> Handle<Mesh>,
) -> anyhow::Result<()> {
    let primitives = world.get_resource::<Primitives>().unwrap();
    let mesh = shape(primitives);
    let material = primitives.default_material.clone();

    world
        .entity_mut(root)
        .insert_bundle(PbrPrimitiveBundle::default())
        .insert_bundle((mesh, material));

    Ok(())
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[uuid = "8b935cbf-5eeb-486b-a54c-7668b95c022c"]
pub struct CubePrefab;

impl PrefabData for CubePrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        common_construct(world, root, |primitives| primitives.cube.clone())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[uuid = "f8f8ca94-5470-4014-b350-66e45fb8a700"]
pub struct UVSpherePrefab;

impl PrefabData for UVSpherePrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        common_construct(world, root, |primitives| primitives.uv_sphere.clone())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[uuid = "fdf29f2c-fc67-4654-8341-e2c415defef1"]
pub struct PlanePrefab;

impl PrefabData for PlanePrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        common_construct(world, root, |primitives| primitives.plane.clone())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[uuid = "12a3f44b-4fe7-4411-9100-0594caa0f3c2"]
pub struct CapsulePrefab;

impl PrefabData for CapsulePrefab {
    fn construct(&self, world: &mut World, root: Entity) -> anyhow::Result<()> {
        common_construct(world, root, |primitives| primitives.capsule.clone())
    }
}

// TODO: CylinderPrefab
// TODO: TorusPrefab

///////////////////////////////////////////////////////////////////////////////

pub fn register_primitives_prefabs(app_builder: &mut AppBuilder) {
    let primitives = Primitives::from_world(&mut app_builder.app.world);
    app_builder
        .insert_resource(primitives)
        .register_prefab::<CubePrefab>(false)
        .register_prefab::<UVSpherePrefab>(false)
        .register_prefab::<PlanePrefab>(false)
        .register_prefab::<CapsulePrefab>(false);
}
