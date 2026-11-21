Attributes to attach to the generated layer type or its fields.

```ignore
#[config(layer_attr(...))]
```

Specify attributes that should be attached to the layer struct definition.
For example, `#[config(layer_attr(derive(Clone)))]` can be used to make
the layer type implement `Clone`.

This attribute can also be applied to struct fields, in which case the
attributes are attached to the corresponding field of the layer type.

Example:

```
use confique::Config;

#[derive(Config)]
#[config(layer_attr(derive(Clone, Debug)))]
struct Conf {
    #[config(layer_attr(serde(alias = "hostname")))]
    host: String,
}

let layer = ConfLayer { host: Some("localhost".into()) };
println!("{:?}", layer.clone());
```
