use std::any::type_name;

use bevy::{
    ecs::{component::Component, entity::MapEntities},
    prelude::*,
    reflect::TypeUuid,
    render::render_graph::base::MainPass,
};
use serde::Deserialize;

use crate::{
    data::BlankPrefab,
    de::PrefabDeserializer,
    manager::{prefab_commit_startup_system, prefab_managing_system},
    prelude::BoxedPrefabData,
    registry::{
        shorten_name, ComponentDescriptorRegistry, ComponentEntityMapperRegistry,
        PrefabDescriptorRegistry,
    },
    Prefab, PrefabConstruct, PrefabData, PrefabNotInstantiatedTag, PrefabTransformOverride,
    PrefabTypeUuid,
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

    fn register_prefab_internal_components(&self, app_builder: &mut AppBuilder) {
        let mut component_registry = app_builder
            .app
            .world
            .get_resource_mut::<ComponentDescriptorRegistry>()
            .unwrap();

        component_registry
            .register_private::<Handle<Prefab>>("Handle<Prefab>".to_string())
            .unwrap();

        component_registry
            .register_private::<PrefabNotInstantiatedTag>("PrefabNotInstantiatedTag".to_string())
            .unwrap();

        component_registry
            .register_private::<PrefabTransformOverride>("PrefabTransformOverride".to_string())
            .unwrap();

        component_registry
            .register_private::<PrefabConstruct>("PrefabConstruct".to_string())
            .unwrap();

        component_registry
            .register_private::<PrefabTypeUuid>("PrefabTypeUuid".to_string())
            .unwrap();
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

        // add prefab manager system
        app_builder
            .add_startup_system(prefab_commit_startup_system.exclusive_system())
            .add_startup_system(prefab_managing_system.exclusive_system())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                prefab_managing_system.exclusive_system(),
            );

        // TODO: avoid getting the same resources multiple times, to reduce startup times
        // register bevy default components
        app_builder
            .register_prefab_mappable_component::<Parent>()
            .register_prefab_component_non_serializable::<GlobalTransform>()
            .register_prefab_component::<Transform>()
            .register_prefab_component::<MainPass>()
            .register_prefab_component::<Draw>()
            .register_prefab_component::<Visible>()
            .register_prefab_component::<RenderPipelines>()
            .register_prefab_component::<PointLight>()
            .register_prefab_component::<DirectionalLight>()
            .register_prefab_component_aliased::<Handle<Mesh>>("Mesh".to_string())
            .register_prefab_component::<Handle<StandardMaterial>>();

        // register components needed by the prefab system
        self.register_prefab_internal_components(app_builder);

        if self.primitives_prefabs {
            crate::builtin::primitives::register_primitives_prefabs(app_builder);
        }

        if self.objects_prefabs {
            crate::builtin::objects::register_objects_prefabs(app_builder);
        }

        // Commit changes
        let world = &mut app_builder.app.world;
        let prefab_registry = world.remove_resource::<PrefabDescriptorRegistry>().unwrap();
        let component_registry = world
            .remove_resource::<ComponentDescriptorRegistry>()
            .unwrap();
        let component_entity_mapper = world
            .remove_resource::<ComponentEntityMapperRegistry>()
            .unwrap();
        let prefab_deserializer =
            PrefabDeserializer::new(component_entity_mapper, component_registry, prefab_registry);
        world.insert_resource(prefab_deserializer);
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

    fn register_prefab_component_non_serializable<C>(self) -> Self
    where
        C: Component + Default + Clone,
    {
        self.register_prefab_component_aliased_non_serializable::<C>(shorten_name(type_name::<C>()))
    }

    fn register_prefab<P>(self, source_prefab_required: bool) -> Self
    where
        P: PrefabData
            + TypeUuid
            + Default
            + Clone
            + Send
            + Sync
            + for<'de> Deserialize<'de>
            + 'static,
    {
        self.register_prefab_aliased::<P>(shorten_name(type_name::<P>()), source_prefab_required)
    }

    fn register_prefab_mappable_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + MapEntities + Clone + for<'de> Deserialize<'de> + 'static;

    fn register_prefab_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + Clone + for<'de> Deserialize<'de> + 'static;

    fn register_prefab_component_aliased_non_serializable<C>(self, alias: String) -> Self
    where
        C: Component + Default + Clone;

    fn register_prefab_aliased<P>(self, alias: String, source_prefab_required: bool) -> Self
    where
        P: PrefabData
            + TypeUuid
            + Default
            + Clone
            + Send
            + Sync
            + for<'de> Deserialize<'de>
            + 'static;
}

impl PrefabAppBuilder for &mut AppBuilder {
    fn register_prefab_mappable_component_aliased<C>(self, alias: String) -> Self
    where
        C: Component + MapEntities + Clone + for<'de> Deserialize<'de> + 'static,
    {
        let builder = self.register_prefab_component_aliased::<C>(alias);

        let mut component_entity_mapper_registry = builder
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
        let mut component_registry = self
            .app
            .world
            .get_resource_mut::<ComponentDescriptorRegistry>()
            .unwrap();

        component_registry
            .register::<C>(alias)
            .expect("prefab component couldn't be registered");

        self
    }

    fn register_prefab_component_aliased_non_serializable<C>(self, alias: String) -> Self
    where
        C: Component + Default + Clone,
    {
        let mut component_registry = self
            .app
            .world
            .get_resource_mut::<ComponentDescriptorRegistry>()
            .unwrap();

        component_registry
            .register_non_serializable::<C>(alias)
            .expect("prefab component couldn't be registered");

        self
    }

    fn register_prefab_aliased<P>(self, alias: String, source_prefab_required: bool) -> Self
    where
        P: PrefabData
            + TypeUuid
            + Default
            + Clone
            + Send
            + Sync
            + for<'de> Deserialize<'de>
            + 'static,
    {
        let mut prefab_registry = self
            .app
            .world
            .get_resource_mut::<PrefabDescriptorRegistry>()
            .unwrap();

        prefab_registry
            .register_aliased::<P>(alias.clone(), source_prefab_required)
            .expect("prefab couldn't be registered");

        let mut component_registry = self
            .app
            .world
            .get_resource_mut::<ComponentDescriptorRegistry>()
            .unwrap();

        component_registry
            .register_prefab_data::<P>(alias)
            .expect("prefab data component couldn't be registered");

        self
    }
}
