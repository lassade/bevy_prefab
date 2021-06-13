use anyhow::Result;
use bevy::{asset::AssetServerSettings, ecs::entity::EntityMap, prelude::*, reflect::TypeUuid};
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
    fn construct(&self, world: &mut World, root_entity: Entity) -> Result<()> {
        let _ = world;
        let _ = root_entity;
        // TODO: point light should be available for this function but it isn't
        Ok(())
    }

    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<()> {
        self.light_entity = entity_map.get(self.light_entity)?;
        Ok(())
    }
}

fn blinking_light_update(
    blinking_lights: Query<&BlinkingLightPrefab>,
    point_lights: Query<&PointLight>,
) {
    for blinking_light in blinking_lights.iter() {
        info!(
            "blinking_light have point light: {}",
            point_lights.get(blinking_light.light_entity).is_ok()
        );
    }
}
