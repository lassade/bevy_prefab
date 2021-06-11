use std::{any::TypeId, collections::hash_map::Entry};

use anyhow::Result;
use bevy::{
    ecs::entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    math::prelude::*,
    prelude::warn,
    reflect::{Reflect, ReflectRef, Struct},
    utils::HashMap,
};
use serde::{
    de::{self, DeserializeSeed, MapAccess},
    Deserialize, //Serialize,
};

///////////////////////////////////////////////////////////////////////////////

pub trait Override {
    fn apply(&self, target: &mut dyn Reflect);
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError>;
}

macro_rules! primitive_data_override {
    ($t:ty) => {
        impl Override for $t {
            fn apply(&self, target: &mut dyn Reflect) {
                if let Some(target) = target.downcast_mut::<$t>() {
                    *target = *self;
                } else {
                    todo!("invalid override")
                }
            }

            fn map_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
                Ok(())
            }
        }
    };
}

primitive_data_override!(u8);
primitive_data_override!(i8);
primitive_data_override!(u16);
primitive_data_override!(i16);
primitive_data_override!(u32);
primitive_data_override!(i32);
primitive_data_override!(u64);
primitive_data_override!(i64);
primitive_data_override!(f32);
primitive_data_override!(f64);

impl Override for Entity {
    fn apply(&self, target: &mut dyn Reflect) {
        if let Some(target) = target.downcast_mut::<Entity>() {
            *target = *self;
        } else {
            todo!("invalid override")
        }
    }

    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        *self = entity_map.get(*self)?;
        Ok(())
    }
}

macro_rules! vector_data_override {
    ($base:ty, $override:tt, $($field:ident,)*) => {
        impl<'de> Deserialize<'de> for $override {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct __Visitor;

                impl<'a, 'de> de::Visitor<'de> for __Visitor {
                    type Value = $override;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("a `")?;
                        formatter.write_str(stringify!($override))?;
                        formatter.write_str("`")
                    }

                    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
                    where
                        A: MapAccess<'de>,
                    {
                        #[allow(non_camel_case_types)]
                        #[derive(Deserialize)]
                        #[serde(field_identifier, rename_all = "lowercase")]
                        enum Field {
                            $($field,)*
                        }

                        $( let mut $field = None; )*

                        while let Some(key) = access.next_key()? {
                            match key {
                                $( Field::$field => $field = Some(access.next_value()?), )*
                            }
                        }

                        Ok($override { $( $field, )* })
                    }
                }

                deserializer.deserialize_struct(stringify!($override), &[$(stringify!($field),)*], __Visitor)
            }
        }

        impl Override for $override {
            fn apply(&self, target: &mut dyn Reflect) {
                if let Some(target) = target.downcast_mut::<$base>() {
                    $(
                        if let Some($field) = self.$field {
                            target.$field = $field;
                        }
                    )*
                } else {
                    todo!("invalid override")
                }
            }

            fn map_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
                Ok(())
            }
        }
    };
}

struct Vec2Override {
    x: Option<f32>,
    y: Option<f32>,
}

struct Vec3Override {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
}

struct Vec4Override {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
    w: Option<f32>,
}

vector_data_override!(Vec2, Vec2Override, x, y,);
vector_data_override!(Vec3, Vec3Override, x, y, z,);
vector_data_override!(Vec4, Vec4Override, x, y, z, w,);
primitive_data_override!(Quat);

///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
enum OverrideDescriptor {
    Field(FieldOverrideDescriptor),
    Struct(StructOverrideDescriptor),
}

impl<'a, 'de> DeserializeSeed<'de> for &'a OverrideDescriptor {
    type Value = Box<dyn Override>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match self {
            OverrideDescriptor::Field(field_overrides) => {
                let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
                (field_overrides.de)(&mut deserializer).map_err(de::Error::custom)
            }
            OverrideDescriptor::Struct(struct_overrides) => {
                deserializer.deserialize_struct("StructOverrides", &[], struct_overrides)
            }
        }
    }
}

#[derive(Clone)]
struct FieldOverrideDescriptor {
    de: fn(&mut dyn erased_serde::Deserializer) -> Result<Box<dyn Override>>,
}

#[derive(Clone)]
struct StructOverrideDescriptor {
    fields: HashMap<String, OverrideDescriptor>,
}

impl<'a, 'de> de::Visitor<'de> for &'a StructOverrideDescriptor {
    type Value = Box<dyn Override>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid `PrefabData` map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut overrides = StructOverride {
            fields: Default::default(),
        };
        while let Some(key) = map.next_key::<String>()? {
            let descriptor = self
                .fields
                .get(&key)
                .ok_or_else(|| de::Error::unknown_field(key.as_str(), &[]))?;

            match overrides.fields.entry(key) {
                Entry::Occupied(occupied) => {
                    return Err(de::Error::custom(format!(
                        "duplicate field `{}`",
                        occupied.key()
                    )));
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(map.next_value_seed(descriptor)?);
                }
            }
        }
        Ok(Box::new(overrides))
    }
}

pub struct StructOverride {
    fields: HashMap<String, Box<dyn Override>>,
}

impl MapEntities for StructOverride {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for (_, v) in &mut self.fields {
            v.map_entities(entity_map)?;
        }
        Ok(())
    }
}

impl Override for StructOverride {
    fn apply(&self, target: &mut dyn Reflect) {
        todo!();
        // for (_, v) in &self.fields {
        //     v.apply(target);
        // }
    }

    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        MapEntities::map_entities(self, entity_map)
    }
}

pub struct PrefabOverrideRegistry {
    registry: HashMap<TypeId, OverrideDescriptor>,
}

impl Default for PrefabOverrideRegistry {
    fn default() -> Self {
        let mut registry = Self {
            registry: Default::default(),
        };

        registry.register::<u8, u8>();
        registry.register::<i8, i8>();
        registry.register::<u16, u16>();
        registry.register::<i16, i16>();
        registry.register::<u32, u32>();
        registry.register::<i32, i32>();
        registry.register::<u64, u64>();
        registry.register::<i64, i64>();
        registry.register::<f32, f32>();
        registry.register::<f64, f64>();
        registry.register::<Entity, Entity>();
        registry.register::<Vec2, Vec2Override>();
        registry.register::<Vec3, Vec3Override>();
        registry.register::<Vec4, Vec4Override>();
        registry.register::<Quat, Quat>();

        registry
    }
}

impl PrefabOverrideRegistry {
    pub fn register<K, T>(&mut self)
    where
        K: 'static,
        T: Override + for<'de> Deserialize<'de> + 'static,
    {
        self.registry.entry(TypeId::of::<K>()).or_insert_with(|| {
            let descriptor = FieldOverrideDescriptor {
                de: |deserializer| Ok(Box::new(T::deserialize(deserializer)?)),
            };
            OverrideDescriptor::Field(descriptor)
        });
    }

    pub fn register_struct<T: Default + Struct>(&mut self) {
        self.register_struct_from_value(&T::default());
    }

    pub fn register_struct_from_value(&mut self, value: &dyn Struct) {
        let mut struct_descriptor = StructOverrideDescriptor {
            fields: Default::default(),
        };

        for (i, field) in value.iter_fields().enumerate() {
            let name = value.name_at(i).unwrap();
            let id = field.type_id();

            let descriptor = if let Some(descriptor) = self.registry.get(&id) {
                descriptor
            } else {
                if let ReflectRef::Struct(inner_value) = field.reflect_ref() {
                    self.register_struct_from_value(inner_value);
                    self.registry.get(&id).unwrap()
                } else {
                    warn!(
                        "field `{}` of `{}` doesn't support overriding",
                        name,
                        value.type_name()
                    );
                    continue;
                }
            };

            struct_descriptor
                .fields
                .insert(name.to_string(), descriptor.clone());
        }

        self.registry.insert(
            value.type_id(),
            OverrideDescriptor::Struct(struct_descriptor),
        );
    }
}
