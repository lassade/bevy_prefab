use anyhow::Result;
use bevy::{
    asset::AssetServerSettings,
    ecs::entity::EntityMap,
    math::{
        curves::{Curve, CurveFixed},
        interpolation::utils::lerp_unclamped,
    },
    prelude::*,
    reflect::TypeUuid,
};
use bevy_prefab::prelude::*;
use rand::prelude::*;
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

struct BlinkingLightLocal {
    curve: CurveFixed<f32>,
}

impl Default for BlinkingLightLocal {
    fn default() -> Self {
        // generate animated noise
        let mut rng = thread_rng();
        let mut samples: Vec<f32> = vec![];
        samples.resize_with(20, || rng.gen());
        let curve = CurveFixed::from_keyframes(20.0, 0, samples);

        Self { curve }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, TypeUuid)]
#[serde(default)]
#[uuid = "0833291b-ecc0-4fff-ae45-42ee8698dd43"]
struct BlinkingLightPrefab {
    //pub color: Color,
    pub min: f32,
    pub max: f32,
    pub speed: f32,
    light_entity: Entity,
}

impl Default for BlinkingLightPrefab {
    fn default() -> Self {
        Self {
            //color: Color::WHITE,
            min: 15.0,
            max: 20.0,
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
    local: Local<BlinkingLightLocal>,
    time: Res<Time>,
    blinking_lights: Query<&BlinkingLightPrefab>,
    mut point_lights: Query<&mut PointLight>,
) {
    for blinking_light in blinking_lights.iter() {
        if let Ok(mut point_light) = point_lights.get_mut(blinking_light.light_entity) {
            let time = (time.seconds_since_startup() as f32 * blinking_light.speed).fract();
            let s = local.curve.sample(time);
            point_light.range = lerp_unclamped(blinking_light.min, blinking_light.max, s);
        }
    }
}
