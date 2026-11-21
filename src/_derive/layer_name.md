Sets the name of the generated layer type.

```ignore
#[config(layer_name = Name)]
```

Sets the name of the generated layer type. By default, it is the name of
the struct with `Layer` appended, e.g. `ConfLayer` for `Conf`. The layer
type is emitted next to the struct (in the same module, with the same
visibility), so this is useful if the default name clashes with another
item or if you just prefer a different name.

```
use confique::{Config, Layer};

#[derive(Config)]
#[config(layer_name = PartialConf)]
struct Conf {
    port: u16,
}

let layer = PartialConf { port: Some(8080) };
assert!(layer.is_complete());
```
