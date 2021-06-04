use std::{
    cell::Cell,
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug},
    sync::Arc,
};

use anyhow::Result;
use bevy::ecs::{component::Component, world::EntityMut};
use parking_lot::RwLock;
use serde::de::DeserializeSeed;
use serde::Deserializer;
use serde::{
    de::{self, EnumAccess, VariantAccess, Visitor},
    Deserialize,
};

// [
//     Entity {
//         id: <stable index u32>,
//         components: [
//             Name(...),
//             Parent(...),
//             Transform { ... },
//             ...
//         ]
//     },
//     Prefab {
//         id: <stable index u32>,
//         source: {
//             uuid: <uuid>,
//             path: <string to prefab asset>,
//         },
//         transform: {
//             position: ,
//             rotation: ,
//             scale: <Optional>,
//         },
//         parent: <some id that isn't self.id>,
//         config: <additional prefab config, used by the prefab construct fn>,
//     },
// ]

#[derive(Default)]
struct ComponentDescriptorRegistryInner {
    named: HashMap<String, ComponentDescriptor>,
}

#[derive(Clone)]
pub struct ComponentDescriptorRegistry {
    lock: Arc<RwLock<ComponentDescriptorRegistryInner>>,
}

impl Default for ComponentDescriptorRegistry {
    fn default() -> Self {
        Self {
            lock: Arc::new(RwLock::new(Default::default())),
        }
    }
}

impl ComponentDescriptorRegistry {
    // pub fn registry<T: 'static>(&self) -> Result<()> {
    //     // TODO: nice name ...
    //     self.registry_aliased();
    // }

    pub fn registry_aliased<T>(&self, alias: String) -> Result<()>
    where
        T: Component + for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let entry = lock.named.entry(alias);
        match entry {
            Entry::Occupied(_) => todo!(),
            Entry::Vacant(vacant) => {
                vacant.insert(ComponentDescriptor {
                    de: &|deserializer, entity| {
                        let value: T = Deserialize::deserialize(deserializer)?;
                        entity.insert(value);
                        Ok(())
                    },
                });
                Ok(())
            }
        }
    }
}

thread_local! {
    static COMPONENT_DESCRIPTOR_REGISTRY: Cell<Option<ComponentDescriptorRegistry>> = Cell::new(None);
}

#[derive(Clone)]
struct ComponentDescriptor {
    de: &'static dyn Fn(&mut dyn erased_serde::Deserializer, &mut EntityMut) -> Result<()>,
    //fields: &'static [&'static str],
}

struct ComponentDescriptorVisitor;

impl<'de> Visitor<'de> for ComponentDescriptorVisitor {
    type Value = ComponentDescriptor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Component`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        COMPONENT_DESCRIPTOR_REGISTRY.with(|cell| {
            let contents = cell.replace(None);
            let descriptor = contents
                .as_ref()
                .and_then(|registry| registry.lock.read().named.get(v).cloned())
                .ok_or_else(|| de::Error::unknown_variant(v, &[]));
            cell.replace(contents);
            descriptor
        })
    }
}

impl<'de> Deserialize<'de> for ComponentDescriptor {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(ComponentDescriptorVisitor)
    }
}

struct ComponentVisitor<'a, 'w> {
    entity: &'a mut EntityMut<'w>,
}

impl<'a, 'w, 'de> DeserializeSeed<'de> for ComponentVisitor<'a, 'w> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Component", &[], self)
    }
}

impl<'a, 'w, 'de> Visitor<'de> for ComponentVisitor<'a, 'w> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Component`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let (descriptor, variant) = data.variant::<ComponentDescriptor>()?;
        let ComponentVisitor { entity } = self;

        // Should only be used if the Component is a enum
        variant.newtype_variant_seed(ComponentData { descriptor, entity })
    }
}

struct ComponentData<'a, 'w> {
    descriptor: ComponentDescriptor,
    entity: &'a mut EntityMut<'w>,
}

impl<'a, 'w, 'de> DeserializeSeed<'de> for ComponentData<'a, 'w> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ComponentData { descriptor, entity } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        (descriptor.de)(&mut deserializer, entity).map_err(de::Error::custom)?;
        Ok(())
    }
}

// Prefab {
//     variant: {
//         uuid: <uuid>,
//         name: <prefab variant name>,
//     }
//     scene: <scene format>
// }

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::World;

    #[derive(Debug, Deserialize)]
    struct Name(String);

    #[test]
    fn read() {
        let registry = ComponentDescriptorRegistry::default();
        registry
            .registry_aliased::<Name>("Name".to_string())
            .unwrap();

        let mut world = World::default();
        let mut entity = world.spawn();
        let input = r#"Name(("Root"))"#;

        COMPONENT_DESCRIPTOR_REGISTRY.with(|cell| {
            cell.replace(Some(registry.clone()));

            let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
            let visitor = ComponentVisitor {
                entity: &mut entity,
            };
            visitor.deserialize(&mut deserializer).unwrap();

            cell.replace(None);
        })
    }
}
