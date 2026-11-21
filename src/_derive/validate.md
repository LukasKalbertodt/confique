Adds a validation check to a field or to the struct.

```ignore
// On fields
#[config(validate = path::to::function)]
// or
#[config(validate(<expr>, "msg"))]

// On the struct
#[config(validate = path::to::function)]
```

# On fields

Adds a validation to the field, i.e. a check that must succeed to be able to
load the configuration. The validator is called as part of the
deserialization, and is thus executed for all layers, not just for the
merged configuration. The attribute can be specified multiple times. For
manually constructed layers, you can run all field validators via
[`Layer::validate`][crate::Layer::validate].

> *Note*: remember ["Parse, don't validate"][parse-not-validate]! If you can
   reasonably represent your validation logic as a type, you should use
   that type instead of validating a weakly-typed field. Example: if your
   config value is an IP-address, use the dedicated `std::net::IpAddr` as
   field type (can be deserialized from strings) instead of a `String`
   field with a `validate` function making sure it's a valid IP-address.
>
> ```ignore
> // GOOD
> addr: std::net::IpAddr,
>
> // BAD
> #[config(validate(addr.parse::<std::net::IpAddr>().is_ok(), "not a valid IP-address"))]
> addr: String,
> ```

[parse-not-validate]: https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/

The `validate = path::to::function` syntax expects a function that is
callable as `Fn(&T) -> Result<(), E>` where `T` is the (non-optional) type
of the field and `E` can be any type implementing `fmt::Display` (e.g. just
`&str`). Example:

```
use confique::Config;

#[derive(Config)]
struct Conf {
    #[config(validate = is_valid_user)]
    user: Option<String>,
}

fn is_valid_user(user: &String) -> Result<(), &'static str> {
    if user == "root" {
        return Err("user 'root' is not allowed");
    }
    if !user.is_ascii() {
        return Err("user must be an ASCII string");
    }
    Ok(())
}
# fn main() {}
```

The `validate(<expr>, "msg")` syntax is only for convenience and intended
for simple cases. It works similar to the `assert!` macro as it expects an
expression validating to `bool` and a string error message. The expression
can access the field value by reference via the field's name. If the
expression validates to `false`, this is treated as a validation error.
Examples:

```
use confique::Config;

#[derive(Config)]
struct Conf {
    #[config(
        validate(!name.is_empty(), "name must not be empty"),
        validate(name.is_ascii(), "name must be ASCII"),
    )]
    name: String,

    #[config(validate(*port >= 1024, "cannot use ports < 1024 as non-root user"))]
    port: Option<u16>,
}
```

For each field with at least one `validate` attribute, the layer type gets
an associated function `validate_{field}(v: &T) -> Result<(), Error>` that
runs all validators of that field. This is useful if you construct layers
manually instead of deserializing them, as validators are then not run
automatically.


# On the struct

Adds a validation to the config struct, i.e. a check that must succeed to be
able to load the configuration. The validator is called inside
`Config::from_layer`, i.e. only after all layers have been merged.

The given`path::to::function` is expected to be a function callable as
`Fn(&T) -> Result<(), E>` where `T` is the struct type (`Self`) and `E` can
be any type implementing `fmt::Display` (e.g. just `&str`). Example:

```
use confique::Config;

#[derive(Config)]
#[config(validate = Self::validate)]
struct ColorMixConfig {
    source_weight: f32,
    target_weight: f32,
}

impl ColorMixConfig {
    fn validate(&self) -> Result<(), &'static str> {
        if self.source_weight + self.target_weight > 1.0 {
            return Err("sum of weights must not exceed 1");
        }
        Ok(())
    }
}
# fn main() {}
```
