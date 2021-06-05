use std::collections::hash_map::Entry;

use anyhow::Result;
use bevy::{ecs::entity::EntityMap, prelude::*, reflect::TypeRegistryArc, utils::HashMap};
use thiserror::Error;

use crate::{
    registry::{ComponentDescriptor, RegistryInner},
    Prefab, PrefabInstance,
};

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum PrefabSpawnError {
    #[error("prefab not found")]
    MissingPrefab(Handle<Prefab>),
    // #[error("prefab contains the unregistered, consider registering it using `app.register_prefab_component::<{0}>()`")]
    // UnregisteredType(&'static str),
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct PrefabManager {
    //instances: HashMap<Handle<Prefab>, Vec<Entity>>,
}

impl PrefabManager {
    fn spawn_internal(
        &mut self,
        world: &mut World,
        parent: Option<Parent>,
        prefabs: &Assets<Prefab>,
        prefab_instance: &PrefabInstance,
        component_registry: &RegistryInner<ComponentDescriptor>,
    ) -> Result<Entity> {
        let prefab = prefabs
            .get(&prefab_instance.source)
            .ok_or_else(|| PrefabSpawnError::MissingPrefab(prefab_instance.source.clone()))?;

        let mut prefab_to_instance = EntityMap::default();

        // Copy prefab entities over
        for archetype in prefab.world.archetypes().iter() {
            for prefab_entity in archetype.entities() {
                let instance_entity = *prefab_to_instance
                    .entry(*prefab_entity)
                    .or_insert_with(|| world.spawn().id());

                for component_id in archetype.components() {
                    let component_info = prefab
                            .world
                            .components()
                            .get_info(component_id)
                            .expect("world must have a `ComponentInfo` for a `ComponentId` of one of their own `Archetype`s");

                    let descriptor = component_registry
                        .find_by_type(component_info.type_id().unwrap())
                        .expect("prefab component type should be registered");

                    (descriptor.copy)(&prefab.world, world, *prefab_entity, instance_entity);
                }
            }
        }

        // Create prefab root entity
        let root_entity = world.spawn().id();

        // Spawn nested prefabs
        for nested_prefab in &prefab.nested_prefabs {
            match prefab_to_instance.entry(nested_prefab.id) {
                Entry::Occupied(_) => todo!(),
                Entry::Vacant(vacant) => {
                    vacant.insert(self.spawn_internal(
                        world,
                        Some(Parent(root_entity)),
                        prefabs,
                        nested_prefab,
                        component_registry,
                    )?);
                }
            }
        }

        // TODO: Map scene components
        // let type_registry = world.get_resource::<TypeRegistryArc>().unwrap().clone();
        // let type_registry = type_registry.read();

        // Start adding data
        let mut root = world.entity_mut(root_entity);
        root.insert(prefab_instance.source.clone());

        // Override prefab transform with instance's transform
        root.insert({
            let mut transform = prefab.transform.clone();
            if let Some(translation) = prefab_instance.transform.translation {
                transform.translation = translation;
            }
            if let Some(rotation) = prefab_instance.transform.rotation {
                transform.rotation = rotation;
            }
            if let Some(scale) = prefab_instance.transform.scale {
                transform.scale = scale;
            }
            transform
        });

        // Override prefab parent
        if let Some(parent) = prefab_instance.parent {
            // TODO: source_to_prefab
            // TODO: prefab_to_instance
            todo!()
        } else if let Some(parent) = parent {
            root.insert(parent);
        }

        // TODO: Put root entities under the prefab root entity

        let prefab_data = &prefab_instance.data.0;
        // Insert the PrefabData (down casted) in the root Entity so it can be available during runtime
        prefab_data.copy_to_instance(&mut root);

        // Run construct prefab function
        prefab_data.construct(world, root_entity)?;

        Ok(root_entity)
    }
}

pub fn prefab_managing_system(world: &mut World) {
    world.resource_scope(|world, mut scene_spawner: Mut<PrefabManager>| {
        // let scene_asset_events = world
        //     .get_resource::<Events<AssetEvent<DynamicScene>>>()
        //     .unwrap();

        // let mut updated_spawned_scenes = Vec::new();
        // for event in scene_spawner
        //     .scene_asset_event_reader
        //     .iter(&scene_asset_events)
        // {
        //     if let AssetEvent::Modified { handle } = event {
        //         if scene_spawner.spawned_dynamic_scenes.contains_key(handle) {
        //             updated_spawned_scenes.push(handle.clone_weak());
        //         }
        //     }
        // }

        // scene_spawner.despawn_queued_scenes(world).unwrap();
        // scene_spawner
        //     .spawn_queued_scenes(world)
        //     .unwrap_or_else(|err| panic!("{}", err));
        // scene_spawner
        //     .update_spawned_scenes(world, &updated_spawned_scenes)
        //     .unwrap();
        // scene_spawner.set_scene_instance_parent_sync(world);
    });
}
