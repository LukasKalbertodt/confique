Assigns an environment variable to this field.

```ignore
#[config(env = "KEY")]
```

Assigns an environment variable to this field. In
[`Layer::from_env`][crate::Layer::from_env], the variable is checked and
deserialized into the field if present.

If the env var is set to an empty string and if the field fails to
parse/deserialize/validate, it is treated as unset.
