use std::{
    cell::Cell,
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug},
    sync::Arc,
};

use anyhow::Result;
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
        T: Debug + for<'de> Deserialize<'de> + 'static,
    {
        let mut lock = self.lock.write();
        let entry = lock.named.entry(alias);
        match entry {
            Entry::Occupied(_) => todo!(),
            Entry::Vacant(vacant) => {
                vacant.insert(ComponentDescriptor {
                    de: &|deserializer| {
                        let value: T = Deserialize::deserialize(deserializer)?;
                        println!("{:?}", value);
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
    de: &'static dyn Fn(&mut dyn erased_serde::Deserializer) -> Result<()>,
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

struct Component;

struct ComponentVisitor {
    seed: (),
}

impl<'de> Visitor<'de> for ComponentVisitor {
    type Value = Component;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Component`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let (descriptor, variant) = data.variant::<ComponentDescriptor>()?;

        struct Seed(ComponentDescriptor);

        impl<'de> DeserializeSeed<'de> for Seed {
            type Value = Component;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
                (self.0.de)(&mut deserializer).map_err(de::Error::custom)?;
                Ok(Component)
            }
        }

        // TODO: Not ideal
        variant.newtype_variant_seed(Seed(descriptor))
    }
}

impl<'de> Deserialize<'de> for Component {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Component", &[], ComponentVisitor { seed: () })
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

    #[derive(Debug, Deserialize)]
    struct Name(String);

    #[test]
    fn read() {
        let registry = ComponentDescriptorRegistry::default();
        registry
            .registry_aliased::<Name>("Name".to_string())
            .unwrap();

        let input = r#"Name(("Root"))"#;

        COMPONENT_DESCRIPTOR_REGISTRY.with(|cell| {
            cell.replace(Some(registry.clone()));
            let _: Component = ron::de::from_str(input).unwrap();
            cell.replace(None);
        })
    }
}
