use bevy::{ecs::entity::EntityMap, prelude::*, utils::HashSet};
use thiserror::Error;

use crate::{
    registry::{
        ComponentDescriptor, ComponentDescriptorRegistry, ComponentEntityMapperRegistry,
        ComponentEntityMapperRegistryInner, RegistryInner,
    },
    Prefab, PrefabConstruct, PrefabInstanceTransform, PrefabNotInstantiatedTag,
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
    component_mapper: &ComponentEntityMapperRegistryInner,
    component_registry: &RegistryInner<ComponentDescriptor>,
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

            for instance_entity in prefab_to_instance.values() {
                let mut instance = world.entity_mut(instance_entity);

                // Map entities components to instance space
                component_mapper.map_entity_components(&mut instance, &prefab_to_instance);

                // Parent all root prefab entities under the instance root
                if instance.get::<Parent>().is_none() {
                    instance.insert(Parent(root_entity));
                }
            }

            let mut root = world.entity_mut(root_entity);

            // Clear not instantiate tag
            root.remove::<PrefabNotInstantiatedTag>();

            // Override prefab transformations with instance's transform
            let mut transform = prefab.transform.clone();
            if let Some(transform_overrides) = root.remove::<PrefabInstanceTransform>() {
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

pub fn prefab_managing_system(world: &mut World) {
    let mut prefabs_queue = vec![];

    // Avoid extra working or using resource scope every frame if none prefabs
    enqueue_prefab_not_instantiated(world, &mut prefabs_queue);

    if prefabs_queue.is_empty() {
        return;
    }

    world.resource_scope(|world, prefabs: Mut<Assets<Prefab>>| {
        world.resource_scope(|world, components: Mut<ComponentDescriptorRegistry>| {
            world.resource_scope(
                |world, component_mapper: Mut<ComponentEntityMapperRegistry>| {
                    //
                    prefab_spawner(
                        world,
                        &*prefabs,
                        &mut prefabs_queue,
                        &*component_mapper.lock.read(),
                        &*components.lock.read(),
                    )
                },
            );
        });
    });
}
