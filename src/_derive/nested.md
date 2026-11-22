Marks the field as a nested configuration.

```ignore
#[config(nested)]
```

Nested fields contain a nested configuration object. The type of this field
must implement `Config`. `Option<_>` as type for nested fields is not allowed.

[`default`](crate::_derive::default), [`env`](crate::_derive::env),
[`parse_env`](crate::_derive::parse_env),
[`deserialize_with`](crate::_derive::deserialize_with) and
[`validate`](crate::_derive::validate) cannot be used on nested fields.
