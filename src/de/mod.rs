use std::{fmt, sync::Arc};

use bevy::{
    ecs::{
        entity::{Entity, EntityMap},
        world::World,
    },
    utils::HashSet,
};
use rand::{prelude::ThreadRng, RngCore};
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    registry::{
        ComponentDescriptorRegistry, ComponentEntityMapperRegistry, PrefabDescriptor,
        PrefabDescriptorRegistry,
    },
    BoxedPrefabData, Prefab,
};

mod component;
mod instance;

use component::IdentifiedComponentSeq;
use instance::IdentifiedInstanceSeq;

///////////////////////////////////////////////////////////////////////////////

struct PrefabVariant<'a> {
    prefab_registry: &'a PrefabDescriptorRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for PrefabVariant<'a> {
    type Value = PrefabDescriptor;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'a, 'de> Visitor<'de> for PrefabVariant<'a> {
    type Value = PrefabDescriptor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab` identifier")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let PrefabVariant {
            prefab_registry: registry,
        } = self;
        match registry.find_by_name(v).cloned() {
            Some(descriptor) => Ok(descriptor),
            None => Err(de::Error::unknown_variant(v, &[])),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) struct IdValidation {
    random: ThreadRng,
    collection: HashSet<Entity>,
}

impl IdValidation {
    pub fn empty() -> Self {
        Self {
            random: rand::thread_rng(),
            collection: HashSet::default(),
        }
    }

    pub fn validate(&mut self, id: Entity) -> bool {
        self.collection.insert(id)
    }

    pub fn generate_unique(&mut self) -> Entity {
        loop {
            let id = Entity::new(self.random.next_u32());
            if self.validate(id) {
                return id;
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

struct PrefabDataDeserializer {
    descriptor: PrefabDescriptor,
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabDataDeserializer {
    type Value = BoxedPrefabData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrefabDataDeserializer { descriptor } = self;
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(deserializer);
        (descriptor.de)(&mut deserializer).map_err(de::Error::custom)
    }
}

///////////////////////////////////////////////////////////////////////////////

struct PrefabBody<'a> {
    descriptor: PrefabDescriptor,
    component_entity_mapper: &'a ComponentEntityMapperRegistry,
    component_registry: &'a ComponentDescriptorRegistry,
    prefab_registry: &'a PrefabDescriptorRegistry,
}

impl<'a, 'de> Visitor<'de> for PrefabBody<'a> {
    type Value = Prefab;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a `Prafab`")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Id,
            Transform,
            Data,
            Components,
            Scene,
        }

        let mut id = None;
        let mut source_to_prefab = EntityMap::default();
        let mut data = None;
        let mut transform = None;
        let mut world = World::default();
        let root_entity = world.spawn().id();

        let PrefabBody {
            component_entity_mapper,
            descriptor,
            component_registry,
            prefab_registry,
        } = self;

        let id_validation = &mut IdValidation::empty();

        // root entity is used hold component data and
        let data_seed = PrefabDataDeserializer { descriptor };

        while let Some(key) = access.next_key()? {
            match key {
                Field::Id => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    let temp = access.next_value()?;
                    if id_validation.validate(temp) {
                        id = Some(temp);
                    } else {
                        return Err(de::Error::custom(format!("conflicting id `{}`", temp.id())));
                    }
                }
                Field::Transform => {
                    if transform.is_some() {
                        return Err(de::Error::duplicate_field("transform"));
                    }
                    transform = Some(access.next_value()?);
                }
                Field::Data => {
                    if data.is_some() {
                        return Err(de::Error::duplicate_field("data"));
                    }
                    data = Some(access.next_value_seed(&data_seed)?);
                }
                Field::Components => access.next_value_seed(IdentifiedComponentSeq {
                    entity_builder: &mut world.entity_mut(root_entity),
                    component_registry,
                })?,
                Field::Scene => {
                    access.next_value_seed(IdentifiedInstanceSeq {
                        id_validation,
                        source_to_prefab: &mut source_to_prefab,
                        world: &mut world,
                        component_registry,
                        prefab_registry,
                    })?;
                }
            }
        }

        let id = id.unwrap_or_else(|| id_validation.generate_unique());
        source_to_prefab.insert(id, root_entity);

        // map entities from source file to prefab space
        component_entity_mapper
            .map_world_components(&mut world, &source_to_prefab)
            .map_err(de::Error::custom)?;

        let transform = transform.unwrap_or_default();
        let mut data = data.unwrap_or_else(|| (data_seed.descriptor.default)());

        // map entities inside the data
        data.0
            .map_entities(&source_to_prefab)
            .map_err(de::Error::custom)?;

        Ok(Prefab {
            root_entity,
            data,
            transform,
            world,
        })
    }
}

///////////////////////////////////////////////////////////////////////////////

const PREFAB_FIELDS: &'static [&'static str] = &["id", "transform", "data", "components", "scene"];

pub(crate) struct PrefabDeserializerInner {
    pub component_entity_mapper: ComponentEntityMapperRegistry,
    pub component_registry: ComponentDescriptorRegistry,
    pub prefab_registry: PrefabDescriptorRegistry,
}

#[derive(Clone)]
pub(crate) struct PrefabDeserializer {
    // TODO: change to Arc<AtomicCell<...>> to support scripting hot-reloading
    pub inner: Arc<PrefabDeserializerInner>,
}

impl PrefabDeserializer {
    pub fn new(
        component_entity_mapper: ComponentEntityMapperRegistry,
        component_registry: ComponentDescriptorRegistry,
        prefab_registry: PrefabDescriptorRegistry,
    ) -> Self {
        Self {
            inner: Arc::new(PrefabDeserializerInner {
                component_entity_mapper,
                component_registry,
                prefab_registry,
            }),
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &'a PrefabDeserializer {
    type Value = Prefab;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_enum("Prefab", &[], self)
    }
}

impl<'a, 'de> Visitor<'de> for &'a PrefabDeserializer {
    type Value = Prefab;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a registered `Prefab`")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let PrefabDeserializerInner {
            component_entity_mapper,
            component_registry,
            prefab_registry,
        } = &*self.inner;

        let (descriptor, variant) = data.variant_seed(PrefabVariant { prefab_registry })?;
        variant.struct_variant(
            PREFAB_FIELDS,
            PrefabBody {
                descriptor,
                component_entity_mapper,
                component_registry,
                prefab_registry,
            },
        )
    }
}
