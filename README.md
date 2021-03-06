# Prefab system for bevy

**Disclaimer** this crate is very experimental, it might not even work, use at your own risk

## Features

1. Human readable format
2. Customizable prefab types
3. Data driven
4. Non-blocking asset loading (asset serialization still locks, but that's inherited from bevy)
5. Prefab data overrides with a custom level of granularity

## Format Overview

The example is given in `ron` file format, but the prefab system can be (de)serialized in any format implemented for `serde`

```json5
Prefab (
    // (optional) prefab root id allows the root entity to be referenced by their children
    id: 9000,
    // (optional) prefab data, it will be inserted as a component of the prefab root entity
    data: (),
    // (optional) prefab root transform
    transform: (),
    // (optional) extra components, easily modify the prefab behaviour and reduces data duplication
    components: []
    // list of instances
    scene: [
        // entity instance
        Entity (
            // stable index, to map entities from file to prefab to instance spaces
            id: 67234,
            // entity components
            components: [
                Name(("Root")),
                Transform(( translation: (0, 0, -10) )),
                // double parenthesis aren't redundant because enums variants
                MyEnum(VariantFoo( 0, 0, 0 )),
            ]
        ),
        // prefab variant instance
        LampPrefab (
            // its possible to omit id's if no-one is referring to this instance
            id: 95649,
            // prefab kind or implementation (what kind of lamp this instance is?)
            source: External("prefabs/flashlight.prefab"),
            // (optional) define parent, when not present the parent will be the root entity
            parent: Some(67234),
            // (optional) prefab instance do override out of the box the [`Transform`] and [`Parent`] components
            transform: (
                position: Some((0, 2, -2)),
                rotation: None,
                scale: None,
            ),
            // (optional) prefab data used to modify this instance, source prefab defaults are used when missing
            overrides: (
                light_color: Rgba( red: 1, green: 0, blue: 0, alpha: 1),
                light_strength: 2,
            ),
            // (optional) override prefab components, except for `Parent` and `Transform` that have a custom override path
            components: [],
        ),
        // fully procedural prefab
        CubePrefab (
            transform: ( position: Some((0, 1, 0), ),
            overrides: ( radius: 2, )
        )
    ],
)
```

## TODO

- (test) top level prefab components
- (test) prefab components overrides
- (behaviour) missing a entity won't result in error, instead a entity tagged with `PrefabMissingEntityTag` is spawned to take their place
- (test) missing entity
- (test) unknown component
- (behaviour) query about prefab loading status
- (behaviour) remove entities if prefab fails to load
- (test) fail to load nested prefab
- (feature) custom dependency tracking until distill
- (behaviour) only spawn a prefab when all their prefabs dependencies where loaded
- (test) modify or change nested prefab inside the construct function
- (feature) send prefab events instantiated or modified
- (feature) hot reload
- (feature) serialization
- (feature) uuid support for prefab variants and component names
- (feature) embedded assets
- (feature) save and load the table of components uuids to be used by non human readable formats on publishing
- (feature) editor
- (feature) benches
- (pref) cache copy functions by archetype

## Usage

```rust
use bevy::prelude::*;
use bevy_prefab::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(
            PrefabPlugin::default()
                // optional pre built-prefabs
                .with_primitives_prefabs()
                .with_objects_prefabs()
        )
        .run();
}
```

## Notes

- prefab data is a component added to the prefab root entity so you can added it to `app.register_prefab_mapped_component<MyPrefabData>()`
to be able to refer to other entities inside the prefab space, keep in mind that prefab instances can't reference
their children for now