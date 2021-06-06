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
            source: External("prefabs/flashlight.prefab"),
            // (optional) prefab instance do override out of the box the [`Transform`] and [`Parent`] components
            transform: (
                position: Some((0, 0, 0)),
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
    ],
)
```

## TODO

- loader
- id validation
- default unique id
- reference instances in prefab data
- fully procedural prefabs (no need for a source prefab file)
- tests
- better commands instancing api ()
- serialization
- uuid support for prefab variants and component names
- embedded assets