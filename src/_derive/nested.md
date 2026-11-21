Marks the field as a nested configuration.

```ignore
#[config(nested)]
```

Nested fields contain a nested configuration object. The type of this field
must implement `Config`. As implied by the previous statement, `Option<_>` as
type for nested fields is not allowed.
