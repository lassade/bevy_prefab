use std::{
    any::{type_name, TypeId},
    collections::hash_map::Entry,
};

use anyhow::Result;
use bevy::{
    asset::Asset,
    ecs::entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    math::prelude::*,
    prelude::{warn, Handle, Hsla, LinSrgba, Mesh, Srgba, StandardMaterial},
    reflect::{Reflect, ReflectMut, ReflectRef, Struct},
    utils::HashMap,
};
use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess},
    Deserialize,
};

///////////////////////////////////////////////////////////////////////////////

pub trait Override: Send + Sync + 'static {
    fn apply_override(&self, target: &mut dyn Reflect);
    fn map_overwritten_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError>;
    fn clone_as_boxed_override(&self) -> Box<dyn Override>;
}

impl Clone for Box<dyn Override> {
    #[inline]
    fn clone(&self) -> Self {
        self.clone_as_boxed_override()
    }
}

impl MapEntities for Box<dyn Override> {
    #[inline]
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.map_overwritten_entities(entity_map)
    }
}

impl Override for () {
    fn apply_override(&self, _: &mut dyn Reflect) {}

    fn map_overwritten_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
        Ok(())
    }

    fn clone_as_boxed_override(&self) -> Box<dyn Override> {
        Box::new(*self)
    }
}

macro_rules! primitive_data_override {
    ($t:ty) => {
        impl Override for $t {
            fn apply_override(&self, target: &mut dyn Reflect) {
                if let Some(target) = target.downcast_mut::<$t>() {
                    *target = *self;
                } else {
                    // TODO: apply_override warnings need a better source identification
                    warn!(
                        "`{}` can't be overwritten by `{}`",
                        target.type_name(),
                        stringify!($t)
                    );
                }
            }

            fn map_overwritten_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
                Ok(())
            }

            fn clone_as_boxed_override(&self) -> Box<dyn Override> {
                Box::new(*self)
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
    fn apply_override(&self, target: &mut dyn Reflect) {
        if let Some(target) = target.downcast_mut::<Entity>() {
            *target = *self;
        } else {
            warn!("`{}` can't be overwritten by `Entity`", target.type_name());
        }
    }

    fn map_overwritten_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        *self = entity_map.get(*self)?;
        Ok(())
    }

    fn clone_as_boxed_override(&self) -> Box<dyn Override> {
        Box::new(*self)
    }
}

impl<T: Asset> Override for Handle<T> {
    fn apply_override(&self, target: &mut dyn Reflect) {
        if let Some(target) = target.downcast_mut::<Handle<T>>() {
            *target = self.clone();
        } else {
            warn!(
                "`{}` can't be overwritten by `Handle<{}>`",
                target.type_name(),
                type_name::<T>()
            );
        }
    }

    fn map_overwritten_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
        Ok(())
    }

    fn clone_as_boxed_override(&self) -> Box<dyn Override> {
        Box::new(self.clone())
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

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: SeqAccess<'de>,
                    {
                        let mut len = 0;
                        $(
                            let $field = seq.next_element()?;
                            len += 1;
                        )*
                        while let Some(de::IgnoredAny) = seq.next_element()? {
                            return Err(de::Error::invalid_length(len, &stringify!($base)));
                        }
                        Ok($override { $( $field, )* })
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
            fn apply_override(&self, target: &mut dyn Reflect) {
                if let Some(target) = target.downcast_mut::<$base>() {
                    $(
                        if let Some($field) = self.$field {
                            target.$field = $field;
                        }
                    )*
                } else {
                    warn!(
                        "`{}` can't be overwritten by `{}`",
                        target.type_name(),
                        stringify!($override)
                    );
                }
            }

            fn map_overwritten_entities(&mut self, _: &EntityMap) -> Result<(), MapEntitiesError> {
                Ok(())
            }

            fn clone_as_boxed_override(&self) -> Box<dyn Override> {
                Box::new(self.clone())
            }
        }
    };
}

/// Overrides each field of a [`Vec2`] individually
#[derive(Clone)]
struct Vec2Override {
    x: Option<f32>,
    y: Option<f32>,
}

/// Overrides each field of a [`Vec3`] individually
#[derive(Clone)]
struct Vec3Override {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
}

/// Overrides each field of a [`Vec4`] individually
#[derive(Clone)]
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

primitive_data_override!(LinSrgba);
primitive_data_override!(Srgba);
primitive_data_override!(Hsla);

///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub enum OverrideDescriptor {
    Field(FieldOverrideDescriptor),
    Struct(StructOverrideDescriptor),
}

impl OverrideDescriptor {
    pub fn blank() -> Self {
        OverrideDescriptor::Field(FieldOverrideDescriptor {
            de: |deserializer| {
                de::IgnoredAny::deserialize(deserializer)?;
                Ok(Box::new(()))
            },
        })
    }
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
pub struct FieldOverrideDescriptor {
    de: fn(&mut dyn erased_serde::Deserializer) -> Result<Box<dyn Override>>,
}

#[derive(Clone)]
pub struct StructOverrideDescriptor {
    fields: HashMap<String, OverrideDescriptor>,
}

impl<'a, 'de> de::Visitor<'de> for &'a StructOverrideDescriptor {
    type Value = Box<dyn Override>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        struct Identifier(String);

        struct IdentifierVisitor;

        impl<'de> de::Visitor<'de> for IdentifierVisitor {
            type Value = Identifier;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid identifier")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Identifier(v.to_string()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Identifier(v))
            }
        }

        impl<'de> Deserialize<'de> for Identifier {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(IdentifierVisitor)
            }
        }

        let mut overrides = StructOverride {
            fields: Default::default(),
        };

        while let Some(Identifier(key)) = map.next_key()? {
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

#[derive(Clone)]
pub struct StructOverride {
    fields: HashMap<String, Box<dyn Override>>,
}

impl Override for StructOverride {
    fn apply_override(&self, target: &mut dyn Reflect) {
        match target.reflect_mut() {
            ReflectMut::Struct(target) => {
                for i in 0..target.field_len() {
                    if let Some(field_override) = self.fields.get(target.name_at(i).unwrap()) {
                        field_override.apply_override(target.field_at_mut(i).unwrap());
                    }
                }
            }
            _ => warn!(
                "`{}` can't be overwritten by `StructOverride`, only struct is supported",
                target.type_name()
            ),
        }
    }

    fn map_overwritten_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for (_, v) in &mut self.fields {
            v.map_entities(entity_map)?;
        }
        Ok(())
    }

    fn clone_as_boxed_override(&self) -> Box<dyn Override> {
        Box::new(self.clone())
    }
}

///////////////////////////////////////////////////////////////////////////////

/// Creates override descriptors that can be used to deserialize and override structs
pub struct OverrideRegistry {
    // TODO: also support uuid lookup in order to support scripting, see src/registry/mod.rs to see an impl example
    registry: HashMap<TypeId, OverrideDescriptor>,
}

impl Default for OverrideRegistry {
    fn default() -> Self {
        let mut registry = Self {
            registry: Default::default(),
        };

        // primitive
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
        // entity
        registry.register::<Entity, Entity>();
        // vector types
        registry.register::<Vec2, Vec2Override>();
        registry.register::<Vec3, Vec3Override>();
        registry.register::<Vec4, Vec4Override>();
        registry.register::<Quat, Quat>();
        // color types
        registry.register::<LinSrgba, LinSrgba>();
        registry.register::<Srgba, Srgba>();
        registry.register::<Hsla, Hsla>();
        // common handle types
        registry.register::<Handle<Mesh>, Handle<Mesh>>();
        registry.register::<Handle<StandardMaterial>, Handle<StandardMaterial>>();

        registry
    }
}

impl OverrideRegistry {
    pub fn find<T: 'static>(&self) -> Option<&OverrideDescriptor> {
        self.find_by_type_id(TypeId::of::<T>())
    }

    pub fn find_by_type_id(&self, type_id: TypeId) -> Option<&OverrideDescriptor> {
        self.registry.get(&type_id)
    }

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

            // TODO: skip private fields

            let descriptor = if let Some(descriptor) = self.registry.get(&id) {
                descriptor
            } else {
                if let ReflectRef::Struct(inner_value) = field.reflect_ref() {
                    self.register_struct_from_value(inner_value);
                    self.registry.get(&id).unwrap()
                } else {
                    warn!(
                        "field `{}` of `{}` doesn't support overriding, consider making the field private or registering it's type with `app.register_prefab_override::<{},{}>()`",
                        name,
                        value.type_name(),
                        field.type_name(),
                        field.type_name(),
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
