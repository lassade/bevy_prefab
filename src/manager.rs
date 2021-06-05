use std::collections::hash_map::Entry;

use anyhow::Result;
use bevy::{ecs::entity::EntityMap, prelude::*};
use thiserror::Error;

use crate::{Prefab, PrefabInstance, PrefabInstanceTransform, PrefabNotInstantiatedTag, registry::{ComponentDescriptor, ComponentDescriptorRegistry, PrefabEntitiesMapperRegistry, PrefabEntityMapperRegistryInner, RegistryInner}};

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum PrefabSpawnError {
    #[error("prefab not found")]
    MissingPrefab(Handle<Prefab>),
    // #[error("prefab contains the unregistered, consider registering it using `app.register_prefab_component::<{0}>()`")]
    // UnregisteredType(&'static str),
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) fn prefab_managing_system(world: &mut World) {
    world.resource_scope(|world, prefabs: Mut<Assets<Prefab>>| {
    world.resource_scope(|world, components: Mut<ComponentDescriptorRegistry>| {
    world.resource_scope(|world, entity_mapper: Mut<PrefabEntitiesMapperRegistry>| {
        let prefabs = &*prefabs;
        let component_registry = &*components.lock.read();
        let entity_mapper = &*entity_mapper.lock.read();
        let mut prefabs_stack = vec![];

        for (entity, handle, tag) in world
            .query::<(Entity, &Handle<Prefab>, &PrefabNotInstantiatedTag)>()
            .iter(world)
        {
            prefabs_stack.push((entity, handle.clone_weak(), tag.0));
        }

        // Order buffer to first instantiate the nested prefabs
        prefabs_stack.sort_by(|(_, _, a), (_, _, b)| a.cmp(b));

        while let Some((root_entity, source_prefab, _)) = prefabs_stack.pop() {
            // TODO: we can not know when a nested prefab finished loading or not
            // TODO: remove PrefabNotInstantiatedTag and add PrefabMissing
            let prefab = match prefabs.get(&source_prefab) {
                Some(prefab) => prefab,
                None => continue,
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
                entity_mapper.map_entity_components(&mut instance, &prefab_to_instance);

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

            //let prefab_data = &prefab_instance.data.0;

            // TODO: Run construct prefab function
            // prefab_data.construct(world, root_entity)?;
        }
    }); }); });
}
