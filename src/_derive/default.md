Sets a default value for this field.

```ignore
#[config(default = ...)]
```

Sets a default value for this field. This is returned by
[`Layer::default_values`][crate::Layer::default_values] and, in most circumstances, used as a
last "layer" to pull values from that have not been set in a layer of
higher-priority. Currently, the following expressions are allowed:

- Booleans, e.g. `default = true`
- Integers, e.g. `default = 900`
- Floats, e.g. `default = 3.14`
- Strings, e.g. `default = "fox"`
- Arrays, e.g. `default = ["foo", "bar"]`
- Key value maps, e.g. `default = { "cat": 3.14, "bear": 9.0 }`

Map keys can be Booleans, integers, floats, and strings. For array and map
values, you can use any of the expressions in the list above (i.e. you
can nest arrays/maps).

The field value is deserialized from the specified default value
(via `serde::de::IntoDeserializer`). So the expression after `default =`
is often not the same Rust type as your field. For example, you can have
`#[config(default = "/foo/bar")]` on the field `path: PathBuf`. This
works fine as `PathBuf` can be deserialized from a string. (Also see the
`IpAddr` field in the [example of the derive macro][macro@crate::Config].)

Example:

```
use std::{net::IpAddr, path::PathBuf};
use confique::Config;

#[derive(Config)]
struct Conf {
    #[config(default = 8080)]
    port: u16,

    #[config(default = "127.0.0.1")]
    bind: IpAddr,

    #[config(default = "/var/log/app.log")]
    log_file: PathBuf,

    #[config(default = ["x-user", "x-password"])]
    headers: Vec<String>,
}

# fn main() -> Result<(), confique::Error> {
// No other sources, so only the default values are used.
let conf = Conf::builder().load()?;
assert_eq!(conf.port, 8080);
assert_eq!(conf.bind, IpAddr::from([127, 0, 0, 1]));
assert_eq!(conf.log_file, PathBuf::from("/var/log/app.log"));
assert_eq!(conf.headers, ["x-user", "x-password"]);
# Ok(())
# }
```

## Integer & float type

If you use an integer or float literal without type suffix, `confique` has
to infer the exact type from the type of the field. This should work in
most cases (`u8`, `f32`, `Vec<i16>`, `[f64; 3]`, ...), but this type
inference is very basic, not even close to what Rust can do. If confique
cannot figure out the type, it defaults to `i32` for integers and `f64`
for floats (like Rust does). If that causes problems for you, just add a
type suffix, e.g. `default = 800u32`.

TODO: example of where suffix is needed
