use bevy::{ecs::entity::EntityMap, prelude::*, utils::HashSet};
use thiserror::Error;

use crate::{
    de::PrefabDeserializer,
    loader::PrefabLoader,
    registry::{ComponentDescriptorRegistry, ComponentEntityMapperRegistry},
    Prefab, PrefabConstruct, PrefabError, PrefabErrorTag, PrefabNotInstantiatedTag,
    PrefabTransformOverride, PrefabTypeUuid,
};

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum PrefabSpawnError {
    #[error("prefab not found")]
    MissingPrefab(Handle<Prefab>),
}

///////////////////////////////////////////////////////////////////////////////

struct Instantiate(Entity, Handle<Prefab>);

fn enqueue_prefab_not_instantiated(world: &mut World, queue: &mut Vec<Instantiate>) {
    for (entity, handle, _) in world
        .query::<(Entity, &Handle<Prefab>, &PrefabNotInstantiatedTag)>()
        .iter(world)
    {
        queue.push(Instantiate(entity, handle.clone_weak()));
    }
}

fn prefab_spawner(
    world: &mut World,
    prefabs: &Assets<Prefab>,
    prefabs_queue: &mut Vec<Instantiate>,
    component_entity_mapper: &ComponentEntityMapperRegistry,
    component_registry: &ComponentDescriptorRegistry,
) {
    let mut blacklist = HashSet::default();

    loop {
        while let Some(Instantiate(root_entity, source_prefab)) = prefabs_queue.pop() {
            // TODO: we can not know when a nested prefab finished loading or not, that causes a lot of issues
            // TODO: remove PrefabNotInstantiatedTag and add PrefabMissing
            let prefab = match prefabs.get(&source_prefab) {
                Some(prefab) => prefab,
                None => {
                    blacklist.insert(root_entity);
                    continue;
                }
            };

            // Validate prefab type with the expected type, sadly this can't be done during
            // de-serialization because the prefab might not be available at that time,
            // so as a consequence the exact source of error will be hard to determine
            let mut root = world.entity_mut(root_entity);
            if let Some(PrefabTypeUuid(uuid)) = root.get() {
                let source = prefab.defaults.0.type_uuid();
                if source != *uuid {
                    // Fail without loading prefab
                    root.remove::<PrefabNotInstantiatedTag>();
                    root.insert(PrefabErrorTag(PrefabError::WrongExpectedSourcePrefab));
                    error!(
                        "prefab expected type `{}` but got source of type `{}`",
                        uuid, source
                    );
                    continue;
                }
            }

            let mut prefab_to_instance = EntityMap::default();

            // Copy prefab entities over
            for archetype in prefab.world.archetypes().iter() {
                for prefab_entity in archetype.entities() {
                    let instance_entity = *prefab_to_instance
                        .entry(*prefab_entity)
                        .or_insert_with(|| world.spawn().id());

                    for component_id in archetype.components() {
                        let component_info =
                            prefab.world.components().get_info(component_id).unwrap();

                        if let Some(descriptor) =
                            component_registry.find_by_type(component_info.type_id().unwrap())
                        {
                            // Copy prefab from his world over the current active world
                            (descriptor.copy)(
                                &prefab.world,
                                world,
                                *prefab_entity,
                                instance_entity,
                            );
                        } else {
                            // Hard error, must be fixed by user
                            panic!(
                                "prefab component `{}` not registered",
                                component_info.name()
                            );
                        }
                    }
                }
            }

            for instance_entity in prefab_to_instance.values() {
                let mut instance = world.entity_mut(instance_entity);

                // Map entities components to instance space
                component_entity_mapper
                    .map_entity_components(&mut instance, &prefab_to_instance)
                    .unwrap();

                // Parent all root prefab entities under the instance root
                if instance.get::<Parent>().is_none() {
                    instance.insert(Parent(root_entity));
                }
            }

            let mut root = world.entity_mut(root_entity);

            // Clear not instantiate tag
            root.remove::<PrefabNotInstantiatedTag>();

            // Use prefab source default if no data is present
            prefab.defaults.0.copy_to_instance(&mut root);

            // Override prefab transformations with instance's transform
            let mut transform = prefab.transform.clone();
            if let Some(transform_overrides) = root.remove::<PrefabTransformOverride>() {
                if let Some(translation) = transform_overrides.translation {
                    transform.translation = translation;
                }
                if let Some(rotation) = transform_overrides.rotation {
                    transform.rotation = rotation;
                }
                if let Some(scale) = transform_overrides.scale {
                    transform.scale = scale;
                }
            }
            root.insert(transform);

            // Run construct function
            if let Some(prefab_construct) = root.remove::<PrefabConstruct>() {
                (prefab_construct.0)(world, root_entity).unwrap();
            } else {
                prefab
                    .defaults
                    .0
                    .construct_instance(world, root_entity)
                    .unwrap();
            }
        }

        enqueue_prefab_not_instantiated(world, prefabs_queue);

        // TODO: very hacky and expensive, we don't know when a prefab was finished loading
        prefabs_queue.retain(|Instantiate(x, _)| !blacklist.contains(x));

        // Nothing left to spawn
        if prefabs_queue.is_empty() {
            break;
        }
    }
}

pub(crate) fn prefab_commit_startup_system(world: &mut World) {
    // Create loader on startup, commits to registered prefab and components
    let loader = PrefabLoader::from_world(world);
    world
        .get_resource::<AssetServer>()
        .unwrap()
        .add_loader(loader);
}

pub fn prefab_managing_system(world: &mut World) {
    let mut prefabs_queue = vec![];

    // Avoid extra working or using resource scope every frame if none prefabs
    enqueue_prefab_not_instantiated(world, &mut prefabs_queue);

    if prefabs_queue.is_empty() {
        return;
    }

    let prefab_registry = world.get_resource::<PrefabDeserializer>().unwrap().clone();

    world.resource_scope(|world, prefabs: Mut<Assets<Prefab>>| {
        prefab_spawner(
            world,
            &*prefabs,
            &mut prefabs_queue,
            &prefab_registry.inner.component_entity_mapper,
            &prefab_registry.inner.component_registry,
        )
    });
}
