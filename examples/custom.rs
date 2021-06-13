use bevy::{
    asset::AssetServerSettings,
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
    reflect::TypeUuid,
};
use bevy_prefab::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    let asset_folder = std::env::current_dir()
        .unwrap()
        .as_path()
        .to_string_lossy()
        .to_string()
        + "/assets";

    App::build()
        .insert_resource(AssetServerSettings { asset_folder })
        .add_plugins(DefaultPlugins)
        .add_plugin(
            PrefabPlugin::default()
                // optional pre built-prefabs
                .with_primitives_prefabs()
                .with_objects_prefabs(),
        )
        .register_prefab::<BlinkingLightPrefab>(true)
        .add_startup_system(setup.system())
        .add_system(blinking_light_update.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    commands.spawn_prefab(asset_server.load("custom_prefab.prefab"));
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[serde(default)]
#[uuid = "0833291b-ecc0-4fff-ae45-42ee8698dd43"]
struct BlinkingLightPrefab {
    pub color: Color,
    pub speed: f32,
    light_entity: Entity,
}

impl Default for BlinkingLightPrefab {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            speed: 0.2,
            light_entity: Entity::new(u32::MAX),
        }
    }
}

impl PrefabData for BlinkingLightPrefab {
    fn construct(&self, world: &mut World, root_entity: Entity) -> anyhow::Result<()> {
        assert!(
            world
                .entity(self.light_entity)
                .get::<PointLight>()
                .is_some(),
            "point light is missing"
        );
        Ok(())
    }
}

impl MapEntities for BlinkingLightPrefab {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        todo!()
    }
}

fn blinking_light_update(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &BlinkingLightPrefab)>,
) {
    // let q = Quat::from_rotation_y(0.5 * time.delta_seconds());
    // for mut transform in query.iter_mut() {
    //     transform.rotation *= q;
    // }
}
