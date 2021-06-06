# Prefab system for bevy

## Features

1. Human readable format
2. Customizable prefab types
3. Data driven

## Format Overview

The example is given in `ron` file format, but the prefab system can be (de)serialized in any format implemented for `serde`

```json5
Prefab (
    // (optional) prefab data, it will be inserted as a component of the prefab root entity
    defaults: (),
    // (optional) prefab root transform
    transform: (),
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
            // (optional) prefab instance do override out of the box the [`Transform`] and [`Parent`] components
            transform: (
                position: Some((0, 2, -2)),
                rotation: None,
                scale: None,
            ),
            // (optional) 
            parent: Some(67234),
            // (optional) prefab data used to modify this instance, source prefab defaults are used when missing
            data: (
                light_color: Rgba( red: 1, green: 0, blue: 0, alpha: 1),
                light_strength: 2,
            ),
        ),
        // fully procedural prefab
        CubePrefab (
            transform: ( position: Some((0, 1, 0), ),
            data: ( radius: 2, )
        )
    ],
)
```

## TODO

- built-in prefab primitives
- prefab components
- tests
- remove entities if prefab fails to load
- better commands (instancing api)
- serialization
- uuid support for prefab variants and component names
- embedded assets

## Usage

```rust
use bevy::prelude::*;
use bevy_prefab::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(
            PrefabPlugin::default()
                // disable bevy built in components (not recommended)
                // .without_builtin_components()
                // optional pre built-prefabs (recommended)
                .with_primitives_prefabs()
        )
        // Yours configs here ...
        .run();
}
```

## Notes

- prefab data is a component added to the prefab root entity so you can added it to `app.register_prefab_mapped_component<MyPrefabData>()`
to be able to refer to other entities inside the prefab space, keep in mind that prefab instances can't reference
their children for now