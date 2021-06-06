use std::any::type_name;

use bevy::{
    ecs::{component::Component, entity::MapEntities},
    prelude::*,
    render::render_graph::base::MainPass,
};
use serde::Deserialize;

use crate::{
    manager::prefab_managing_system,
    registry::{
        shorten_name, ComponentDescriptorRegistry, ComponentEntityMapperRegistry,
        PrefabDescriptorRegistry,
    },
    PrefabData,
};

/// Adds prefab functionality to bevy
#[derive(Default, Debug)]
pub struct PrefabPlugin {
    without_builtin_components: bool,
    primitives_prefabs: bool,
}

impl PrefabPlugin {
    /// **DON'T** add bevy built-in components [`Parent`], [`Transform`], [`Handle<Mesh>`] and so on ...
    pub fn without_builtin_components(mut self) -> Self {
        self.without_builtin_components = true;
        self
    }

    /// Add primitive prefabs such as: `CubePrefab`, `CylinderPrefab` and so on
    pub fn with_primitives_prefabs(mut self) -> Self {
        self.primitives_prefabs = true;
        self
    }
}

impl Plugin for PrefabPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // insert registry resources
        app.insert_resource(PrefabDescriptorRegistry::default())
            .insert_resource(ComponentDescriptorRegistry::default())
            .insert_resource(ComponentEntityMapperRegistry::default());

        // add prefab manager system
        app.add_startup_system(prefab_managing_system.exclusive_system())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                prefab_managing_system.exclusive_system(),
            );

        if self.primitives_prefabs {
            warn!("`PrefabPlugin` primitive prefabs aren't implemented yet");
        }

        // register bevy default components
        if self.without_builtin_components {
            app.register_prefab_mappable_component::<Parent>()
                .register_prefab_component::<Transform>()
                .register_prefab_component::<MainPass>()
                .register_prefab_component::<Draw>()
                .register_prefab_component::<Visible>()
                .register_prefab_component::<RenderPipelines>()
                .register_prefab_component::<PointLight>()
                .register_prefab_component::<DirectionalLight>()
                .register_prefab_component_aliased::<Handle<Mesh>>("Mesh".to_string())
                .register_prefab_component_aliased::<Handle<StandardMaterial>>(
                    "StandardMaterial".to_string(),
                );
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
            .expect("`App` doesn't have the resource `ComponentEntityMapperRegistry`");

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
            .expect("`App` doesn't have the resource `ComponentDescriptorRegistry`");

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
            .expect("`App` doesn't have a the resource `PrefabDescriptorRegistry`");

        prefab_registry
            .register_aliased::<D>(alias)
            .expect("prefab couldn't be registered");

        self
    }
}
