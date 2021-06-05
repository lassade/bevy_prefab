use std::fmt;

use anyhow::Result;
use bevy::ecs::world::EntityMut;
use parking_lot::RwLockReadGuard;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, SeqAccess, VariantAccess, Visitor},
    Deserializer,
};

use crate::registry::{ComponentDescriptor, RegistryInner};

///////////////////////////////////////////////////////////////////////////////

struct ComponentIdentifier<'a> {
    component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentIdentifier<'a> {
    type Value = ComponentDescriptor;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'a, 'de> Visitor<'de> for ComponentIdentifier<'a> {
    type Value = ComponentDescriptor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Component`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let ComponentIdentifier { component_registry } = self;
        component_registry
            .named
            .get(v)
            .cloned()
            .ok_or_else(|| de::Error::unknown_variant(v, &[]))
    }
}

struct ComponentData<'a, 'w> {
    descriptor: ComponentDescriptor,
    entity_builder: &'a mut EntityMut<'w>,
}

impl<'a, 'w, 'de> DeserializeSeed<'de> for ComponentData<'a, 'w> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ComponentData {
            descriptor,
            entity_builder,
        } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        (descriptor.de)(&mut deserializer, entity_builder).map_err(de::Error::custom)?;
        Ok(())
    }
}

struct IdentifiedComponent<'a, 'w> {
    entity_builder: &'a mut EntityMut<'w>,
    component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
}

impl<'a, 'w, 'de> DeserializeSeed<'de> for IdentifiedComponent<'a, 'w> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Component", &[], self)
    }
}

impl<'a, 'w, 'de> Visitor<'de> for IdentifiedComponent<'a, 'w> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Component`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let IdentifiedComponent {
            entity_builder,
            component_registry,
        } = self;
        let (descriptor, variant) =
            data.variant_seed(ComponentIdentifier { component_registry })?;

        // Should only be used if the Component is a enum
        variant.newtype_variant_seed(ComponentData {
            descriptor,
            entity_builder,
        })
    }
}

pub(crate) struct IdentifiedComponentSeq<'a, 'w> {
    pub(crate) entity_builder: &'a mut EntityMut<'w>,
    pub(crate) component_registry: &'a RwLockReadGuard<'a, RegistryInner<ComponentDescriptor>>,
}

impl<'a, 'w, 'de> DeserializeSeed<'de> for IdentifiedComponentSeq<'a, 'w> {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'a, 'w, 'de> Visitor<'de> for IdentifiedComponentSeq<'a, 'w> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `Component` sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let IdentifiedComponentSeq {
            entity_builder,
            component_registry,
        } = self;

        while let Some(_) = seq.next_element_seed(IdentifiedComponent {
            entity_builder,
            component_registry,
        })? {
            // Do nothing, just deserialize all elements in the sequence
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;
    use serde::Deserialize;

    use super::*;
    use crate::registry::ComponentDescriptorRegistry;

    #[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
    struct Name(String);

    #[test]
    fn read() {
        let component_registry = ComponentDescriptorRegistry::default();
        component_registry
            .register_aliased::<Name>("Name".to_string())
            .unwrap();

        let mut world = World::default();
        let mut entity_builder = world.spawn();
        let input = r#"Name(("Root"))"#;

        let mut deserializer = ron::de::Deserializer::from_str(input).unwrap();
        let visitor = IdentifiedComponent {
            entity_builder: &mut entity_builder,
            component_registry: &component_registry.lock.read(),
        };
        visitor.deserialize(&mut deserializer).unwrap();

        let entity_id = entity_builder.id();
        assert_eq!(
            world.get::<Name>(entity_id).cloned(),
            Some(Name("Root".to_string()))
        );
    }
}
