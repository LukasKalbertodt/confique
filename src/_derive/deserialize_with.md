Custom deserialization function for this field.

```ignore
#[config(deserialize_with = path::to::function)]
```

Like [serde's `deserialize_with` attribute][serde-deser].

[serde-deser]: https://serde.rs/field-attrs.html#deserialize_with
