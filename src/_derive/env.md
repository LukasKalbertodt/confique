Assigns an environment variable to this field.

```ignore
#[config(env = "KEY")]
```

Assigns an environment variable to this field. In
[`Layer::from_env`][crate::Layer::from_env], the variable is checked and
deserialized into the field if present.

If the env var is set to an empty string and if the field fails to
parse/deserialize/validate, it is treated as unset.

Example:

```
use confique::Config;

#[derive(Config)]
struct Conf {
    #[config(env = "APP_PORT", default = 8080)]
    port: u16,
}

# fn main() -> Result<(), confique::Error> {
std::env::set_var("APP_PORT", "9000");
let conf = Conf::builder().env().load()?;
assert_eq!(conf.port, 9000);
# Ok(())
# }
```
