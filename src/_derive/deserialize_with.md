Custom deserialization function for this field.

```ignore
#[config(deserialize_with = path::to::function)]
```

Like [serde's `deserialize_with` attribute][serde-deser]: the given function
is used to deserialize the field's value instead of the `Deserialize` impl of
its type. It has to be callable as
`fn<'de, D>(D) -> Result<T, D::Error> where D: Deserializer<'de>`, where `T`
is the (non-optional) type of the field. This is useful for types that don't
implement `Deserialize`, or to accept a different representation than the
default one. serde is reexported as [`confique::serde`][crate::serde], so you
don't need a direct dependency on it.

Unlike with serde, the function is not called for `null` values: as for all
fields, `null` means the value is not set.

[serde-deser]: https://serde.rs/field-attrs.html#deserialize_with

Example:

```
use std::time::Duration;
use confique::{Config, serde::{Deserialize, Deserializer}};

#[derive(Config)]
struct Conf {
    #[config(default = 30, deserialize_with = duration_from_secs)]
    timeout: Duration,
}

fn duration_from_secs<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    u64::deserialize(d).map(Duration::from_secs)
}

# fn main() -> Result<(), confique::Error> {
let conf = Conf::builder().load()?;
assert_eq!(conf.timeout, Duration::from_secs(30));
# Ok(())
# }
```
