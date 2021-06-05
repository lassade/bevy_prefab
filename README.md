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
        Lamp (
            id: 95649,
            // prefab kind or implementation (what kind of lamp this instance is?)
            source: External(
                uuid: "76500818-9b39-4655-9d32-8f1ac0ecbb41",
                path: "prefabs/flashlight.prefab",
            ),
            // (optional) prefab instance do override out of the box the [`Transform`] and [`Parent`] components
            transform: (
                position: (0, 0, 0),
                rotation: None,
                scale: None,
            ),
            // (optional) 
            parent: Some(67234),
            // (optional) prefab data used to modify this instance, source prefab defaults are used when missing
            data: (
                light_color: LinRgba(1, 0, 0, 1),
                light_strength: 2,
            ),
        ),
    ],
)
```

## TODO

1. Testing
2. Better instancing API
3. Embedded assets
4. Serialization
5. Uuid support for Prefab variants and Component names