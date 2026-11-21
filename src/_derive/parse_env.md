Function used to parse environment variables.

```ignore
#[config(parse_env = path::to::function)]
```

Function used to parse environment variables. Mostly useful if you need to
parse lists or other complex objects from env vars. Function needs
signature `fn(&str) -> Result<T, impl std::error::Error>` where `T` is the
type of the field. Can only be present if the [`env`](crate::_derive::env) attribute
is present. Also see [`env::parse`][crate::env::parse].

Example:

```
use confique::Config;

#[derive(Config)]
struct Conf {
    #[config(env = "APP_PORTS", parse_env = confique::env::parse::list_by_comma)]
    ports: Vec<u16>,
}

# fn main() -> Result<(), confique::Error> {
std::env::set_var("APP_PORTS", "80,443");
let conf = Conf::builder().env().load()?;
assert_eq!(conf.ports, [80, 443]);
# Ok(())
# }
```
