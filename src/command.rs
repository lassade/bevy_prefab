use bevy::{ecs::system::Command, prelude::*};

use crate::{Prefab, PrefabNotInstantiatedTag};

struct SpawnPrefab<B> {
    prefab_handle: Handle<Prefab>,
    overrides: B,
}

impl<B> Command for SpawnPrefab<B>
where
    B: Bundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World) {
        let mut root = world.spawn();
        root.insert_bundle((
            self.prefab_handle,
            Transform::default(),
            PrefabNotInstantiatedTag,
        ));
        root.insert_bundle(self.overrides);
    }
}

pub trait PrefabCommands {
    fn spawn_prefab(self, prefab_handle: Handle<Prefab>) -> Self;

    fn spawn_prefab_with_overrides<B>(self, prefab_handle: Handle<Prefab>, overrides: B) -> Self
    where
        B: Bundle + Send + Sync + 'static;
}

impl<'a, 'c> PrefabCommands for &'c mut Commands<'a> {
    fn spawn_prefab(self, prefab_handle: Handle<Prefab>) -> Self {
        self.spawn_prefab_with_overrides(prefab_handle, ())
    }

    fn spawn_prefab_with_overrides<B>(self, prefab_handle: Handle<Prefab>, overrides: B) -> Self
    where
        B: Bundle + Send + Sync + 'static,
    {
        self.add(SpawnPrefab {
            prefab_handle,
            overrides,
        });
        self
    }
}
