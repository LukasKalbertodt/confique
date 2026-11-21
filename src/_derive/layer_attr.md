Attributes to attach to the generated layer type or its fields.

```ignore
#[config(layer_attr(...))]
```

Specify attributes that should be attached to the layer struct definition.
For example, `#[config(layer_attr(derive(Clone)))]` can be used to make
the layer type implement `Clone`.

This attribute can also be applied to struct fields.
