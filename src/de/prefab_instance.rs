use std::fmt;

use anyhow::Result;
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{PrefabDescriptor, RegistryInner},
    BoxedPrefabData, PrefabInstance, PrefabNodeId,
};

///////////////////////////////////////////////////////////////////////////////

struct PrefabIdentifier<'a> {
    registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for PrefabIdentifier<'a> {
    type Value = PrefabDescriptor;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'a, 'de> Visitor<'de> for PrefabIdentifier<'a> {
    type Value = PrefabDescriptor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let PrefabIdentifier { registry } = self;
        registry
            .named
            .get(v)
            .cloned()
            .ok_or_else(|| de::Error::unknown_variant(v, &[]))
    }
}

const PREFAB_INSTANCE_FIELDS: &'static [&'static str] =
    &["id", "source", "transform", "parent", "data"];

struct PrefabIdentifiedInstanceVisitor {
    descriptor: PrefabDescriptor,
}

impl<'de> Visitor<'de> for PrefabIdentifiedInstanceVisitor {
    type Value = PrefabInstance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `PrefabInstance`")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Source,
            Transform,
            Parent,
            Data,
        }

        let mut id = None;
        let mut source = None;
        let mut transform = None;
        let mut parent = None;
        let mut data = None;

        while let Some(key) = access.next_key()? {
            match key {
                Field::Id => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    id = Some(access.next_value::<u32>()?);
                }
                Field::Source => {
                    // TODO:
                    // if source.is_some() {
                    //     return Err(de::Error::duplicate_field("source"));
                    // }
                    // source = Some(access.next_value()?);
                }
                Field::Transform => {
                    if transform.is_some() {
                        return Err(de::Error::duplicate_field("transform"));
                    }
                    transform = Some(access.next_value()?);
                }
                Field::Parent => {
                    // TODO:
                    // if parent.is_some() {
                    //     return Err(de::Error::duplicate_field("parent"));
                    // }
                    // parent = Some(access.next_value()?);
                }
                Field::Data => {
                    if data.is_some() {
                        return Err(de::Error::duplicate_field("data"));
                    }
                    data = Some(access.next_value_seed(&self)?);
                }
            }
        }

        let id = PrefabNodeId(id.ok_or(de::Error::missing_field("id"))?);
        let source = source.unwrap_or_default();
        let parent = parent.unwrap_or_default();
        let transform = transform.unwrap_or_default();
        let data = data.unwrap_or_default();

        Ok(PrefabInstance {
            id,
            source,
            parent,
            transform,
            data,
        })
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabIdentifiedInstanceVisitor {
    type Value = Option<BoxedPrefabData>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrefabIdentifiedInstanceVisitor { descriptor } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        let data = (descriptor.de)(&mut deserializer).map_err(de::Error::custom)?;
        Ok(Some(data))
    }
}

pub(crate) struct PrefabIdentifiedInstance<'a> {
    registry: &'a RwLockReadGuard<'a, RegistryInner<PrefabDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for PrefabIdentifiedInstance<'a> {
    type Value = PrefabInstance;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Prefab", &[], self)
    }
}

impl<'a, 'de> Visitor<'de> for PrefabIdentifiedInstance<'a> {
    type Value = PrefabInstance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let PrefabIdentifiedInstance { registry } = self;
        let (descriptor, variant) = data.variant_seed(PrefabIdentifier { registry })?;

        variant.struct_variant(
            PREFAB_INSTANCE_FIELDS,
            PrefabIdentifiedInstanceVisitor { descriptor },
        )
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;
    use serde::Deserialize;

    use super::*;
    use crate::{registry::PrefabDescriptorRegistry, PrefabData};

    #[derive(Debug, Deserialize, Clone)]
    struct Lamp {
        light_strength: f32,
    }

    impl PrefabData for Lamp {
        fn construct(&self, world: &mut World) -> Result<()> {
            let _ = world;
            todo!()
        }
    }

    #[test]
    fn read() {
        let registry = PrefabDescriptorRegistry::default();
        registry.register::<Lamp>().unwrap();

        let input = r#"Lamp(
            id: 95649,
            //source: (
            //    uuid: "76500818-9b39-4655-9d32-8f1ac0ecbb41",
            //    path: "prefabs/lamp.prefab",
            //),
            transform: (
                position: (0, 0, 0),
                rotation: (0, 0, 0, 1),
                scale: None,
            ),
            //parent: Some(67234),
            data: (
                //light_color: LinRgba(1, 0, 0, 1),
                light_strength: 2,
            ),
        )"#;

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let lock = registry.lock.read();
        let visitor = PrefabIdentifiedInstance { registry: &lock };
        let prefab_instance = visitor.deserialize(&mut deserializer).unwrap();

        panic!("{:?}", prefab_instance);

        // let entity_id = entity_builder.id();
        // assert_eq!(
        //     world.get::<Name>(entity_id).cloned(),
        //     Some(Name("Root".to_string()))
        // );
    }
}
