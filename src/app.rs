use std::any::type_name;

use bevy::{
    ecs::{component::Component, entity::MapEntities},
    prelude::*,
    render::render_graph::base::MainPass,
};
use serde::Deserialize;

use crate::{
    data::BlankPrefab,
    loader::PrefabLoader,
    manager::prefab_managing_system,
    prelude::BoxedPrefabData,
    registry::{
        shorten_name, ComponentDescriptorRegistry, ComponentEntityMapperRegistry,
        PrefabDescriptorRegistry,
    },
    Prefab, PrefabData,
};

/// Adds prefab functionality to bevy
#[derive(Default, Debug)]
pub struct PrefabPlugin {
    primitives_prefabs: bool,
    objects_prefabs: bool,
}

impl PrefabPlugin {
    /// Adds all built in prefabs
    pub fn with_all_builtin_prefabs(self) -> Self {
        Self {
            primitives_prefabs: true,
            objects_prefabs: true,
        }
    }

    /// Add primitive prefabs such as: `CubePrefab`, `CylinderPrefab` and so on
    pub fn with_primitives_prefabs(mut self) -> Self {
        self.primitives_prefabs = true;
        self
    }

    /// Add primitive prefabs such as: `StaticMeshPrefab`, `PointLightPrefab`, `PerspectiveCameraPrefab` and so on
    pub fn with_objects_prefabs(mut self) -> Self {
        self.objects_prefabs = true;
        self
    }
}

impl Plugin for PrefabPlugin {
    fn build(&self, app_builder: &mut AppBuilder) {
        // register prefab asset
        app_builder.add_asset::<Prefab>();

        // add empty prefab resource to be the source for any procedural prefabs
        let mut prefabs = app_builder
            .app
            .world
            .get_resource_mut::<Assets<Prefab>>()
            .unwrap();
        prefabs.set_untracked(
            Handle::<Prefab>::default(),
            Prefab {
                defaults: BoxedPrefabData(Box::new(BlankPrefab)),
                transform: Transform::default(),
                world: World::default(),
            },
        );

        // insert registry resources
        app_builder
            .insert_resource(PrefabDescriptorRegistry::default())
            .insert_resource(ComponentDescriptorRegistry::default())
            .insert_resource(ComponentEntityMapperRegistry::default());

        let loader = PrefabLoader::from_world(&mut app_builder.app.world);
        app_builder.add_asset_loader(loader);

        // add prefab manager system
        app_builder
            .add_startup_system(prefab_managing_system.exclusive_system())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                prefab_managing_system.exclusive_system(),
            );

        // register bevy default components
        app_builder
            .register_prefab_mappable_component::<Parent>()
            .register_prefab_component::<Transform>()
            .register_prefab_component::<MainPass>()
            .register_prefab_component::<Draw>()
            .register_prefab_component::<Visible>()
            .register_prefab_component::<RenderPipelines>()
            .register_prefab_component::<PointLight>()
            .register_prefab_component::<DirectionalLight>()
            .register_prefab_component_aliased::<Handle<Mesh>>("Mesh".to_string())
            .register_prefab_component::<Handle<StandardMaterial>>();

        if self.primitives_prefabs {
            crate::builtin::primitives::register_primitives_prefabs(app_builder);
        }

        if self.objects_prefabs {
            crate::builtin::objects::register_objects_prefabs(app_builder);
        }
    }
}

pub trait PrefabAppBuilder: Sized {
    fn register_prefab_mappable_component<C>(self) -> Self
    where
        C: Component + MapEntities + Clone + for<'de> Deserialize<'de> + 'static,
    {
        self.register_prefab_mappable_component_aliased::<C>(shorten_name(type_name::<C>()))
    }

    fn register_prefab_component<C>(self) -> Self
    where
        C: Component + Clone + for<'de> Deserialize<'de> + 'static,
    {
        self.register_prefab_component_aliased::<C>(shorten_name(type_name::<C>()))
    }

    fn register_prefab<D>(self) -> Self
    where
        D: PrefabData + Default + Clone + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        self.register_prefab_aliased::<D>(shorten_name(type_name::<D>()))
    }

    fn register_prefab_mappable_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + MapEntities + Clone + for<'de> Deserialize<'de> + 'static;

    fn register_prefab_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + Clone + for<'de> Deserialize<'de> + 'static;

    fn register_prefab_aliased<D>(self, alias: String) -> Self
    where
        D: PrefabData + Default + Clone + Send + Sync + for<'de> Deserialize<'de> + 'static;
}

impl PrefabAppBuilder for &mut AppBuilder {
    fn register_prefab_mappable_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + MapEntities + Clone + for<'de> Deserialize<'de> + 'static,
    {
        let builder = self.register_prefab_component_aliased::<C>(alias);

        let component_entity_mapper_registry = builder
            .app
            .world
            .get_resource_mut::<ComponentEntityMapperRegistry>()
            .unwrap();

        component_entity_mapper_registry.register::<C>();

        builder
    }

    fn register_prefab_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + Clone + for<'de> Deserialize<'de> + 'static,
    {
        let component_registry = self
            .app
            .world
            .get_resource_mut::<ComponentDescriptorRegistry>()
            .unwrap();

        component_registry
            .register_aliased::<C>(alias)
            .expect("prefab component couldn't be registered");

        self
    }

    fn register_prefab_aliased<D>(self, alias: String) -> Self
    where
        D: PrefabData + Default + Clone + Send + Sync + for<'de> Deserialize<'de> + 'static,
    {
        let prefab_registry = self
            .app
            .world
            .get_resource_mut::<PrefabDescriptorRegistry>()
            .unwrap();

        prefab_registry
            .register_aliased::<D>(alias)
            .expect("prefab couldn't be registered");

        self
    }
}
