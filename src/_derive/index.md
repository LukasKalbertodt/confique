Detailed documentation of [`#[derive(Config)]`][macro@crate::Config]: all 
attributes and other details.

See [`_example`](crate::_example) for a full example with generated code.

# How to use

The derive only works for structs with named fields (i.e. not for tuple structs,
unit structs, enums, or unions). There are two types of fields distinguished by 
this macro: nested and leaf fields.

- **Nested fields**: they have to be annotated with `#[config(nested)]` and
  contain a nested configuration object. The type of this field must
  implement `Config`. `Option<_>` as type for nested fields is not allowed.

- **Leaf fields**: all fields *not* annotated with `#[config(nested)]`,
  these contain your actual values. The type of such a field has to
  implement `serde::Deserialize` or you have to add a `deserialize_with`
  attribute.

Doc comments on the struct and the individual fields are interpreted and
stored in [`Meta`][crate::meta::Meta]. They are used in the formatting functions
(e.g. `toml::template`).

## Special types for leaf fields

These types give a different meaning/semantic to the field. Please note that
due to the limitations of derive macros, the type is checked *literally*.
So it won't work if you rename symbols or use full paths.

- **`Option<T>`**: this marks the field as an optional field. All other
  fields are non-optional and will raise an error if while loading the
  configuration, no value has been set for them. Optional fields cannot have
  a `#[config(default = ...)]` attribute as that would not make sense. Note:
  the (unqualified) symbol `Option` must be in scope and refer to
  `std::option::Option`.

## Generated layer type

The derive generates a layer type next to your struct, named `{Name}Layer` by
default (see [`layer_name`](crate::_derive::layer_name)) and with the same
visibility as your struct. Apart from implementing [`Layer`](crate::Layer), it
has two associated functions for each leaf field:

- `deserialize_{field}`: deserializes a value for that field like confique
  does when loading a layer, i.e. using
  [`deserialize_with`](crate::_derive::deserialize_with) and running
  [validators](crate::_derive::validate).
- `validate_{field}`: runs all [validators](crate::_derive::validate) of that
  field on a given value (a no-op if there are none).


# Attributes

All attributes are specified via `#[config(...)]`. Depending on the attribute, 
they can apply to either the struct itself, a struct field, or both.

| Attribute | Field | Struct | Description |
| --- | :---: | :---: | --- |
| [`default`](crate::_derive::default) | ✓ | | Default value of the field |
| [`env`](crate::_derive::env) | ✓ | | Load the field from an environment variable |
| [`parse_env`](crate::_derive::parse_env) | ✓ | | Custom function to parse the environment variable |
| [`deserialize_with`](crate::_derive::deserialize_with) | ✓ | | Custom deserialization function |
| [`validate`](crate::_derive::validate) | ✓ | ✓ | Validation checks |
| [`nested`](crate::_derive::nested) | ✓ | | Marks the field as nested configuration |
| [`layer_attr`](crate::_derive::layer_attr) | ✓ | ✓ | Attributes for the generated layer type |
| [`layer_name`](crate::_derive::layer_name) | | ✓ | Name of the generated layer type |
