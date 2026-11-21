Function used to parse environment variables.

```ignore
#[config(parse_env = path::to::function)]
```

Function used to parse environment variables. Mostly useful if you need to
parse lists or other complex objects from env vars. Function needs
signature `fn(&str) -> Result<T, impl std::error::Error>` where `T` is the
type of the field. Can only be present if the [`env`](crate::_derive::env) attribute
is present. Also see [`env::parse`][crate::env::parse].
