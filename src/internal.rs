//! These functions are used by the code generated by the macro, but are not
//! intended to be used directly. None of this is covered by semver! Do not use
//! any of this directly.

use std::fmt::Display;

use crate::{error::ErrorInner, Error};

pub fn deserialize_default<I, O>(src: I) -> Result<O, serde::de::value::Error>
where
    I: for<'de> serde::de::IntoDeserializer<'de>,
    O: for<'de> serde::Deserialize<'de>,
{
    O::deserialize(src.into_deserializer())
}

pub fn into_deserializer<'de, T>(src: T) -> <T as serde::de::IntoDeserializer<'de>>::Deserializer
where
    T: serde::de::IntoDeserializer<'de>,
{
    src.into_deserializer()
}

pub fn unwrap_or_missing_value_err<T>(value: Option<T>, path: &str) -> Result<T, Error> {
    match value {
        Some(v) => Ok(v),
        None => Err(ErrorInner::MissingValue(path.into()).into()),
    }
}

pub fn map_err_prefix_path<T>(res: Result<T, Error>, prefix: &str) -> Result<T, Error> {
    res.map_err(|e| {
        if let ErrorInner::MissingValue(path) = &*e.inner {
            ErrorInner::MissingValue(format!("{prefix}.{path}")).into()
        } else {
            e
        }
    })
}

pub fn do_validate_field<T, E>(t: &T, validate: &dyn Fn(&T) -> Result<(), E>) -> Result<(), String>
where
    E: Display,
{
    validate(t).map_err(|e| format!("validation error: {e}"))
}


macro_rules! get_env_var {
    ($key:expr, $field:expr) => {
        match std::env::var($key) {
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                let err = ErrorInner::EnvNotUnicode {
                    key: $key.into(),
                    field: $field.into(),
                };
                return Err(err.into());
            }
            Ok(s) => s,
        }
    };
}

pub fn from_env<'de, T: serde::Deserialize<'de>>(
    key: &str,
    field: &str,
) -> Result<Option<T>, Error> {
    from_env_with_deserializer(key, field, |de| T::deserialize(de))
}

pub fn from_env_with_parser<T, E: std::error::Error + Send + Sync + 'static>(
    key: &str,
    field: &str,
    parse: fn(&str) -> Result<T, E>,
) -> Result<Option<T>, Error> {
    let v = get_env_var!(key, field);
    let is_empty = v.is_empty();
    match parse(&v) {
        Ok(v) => Ok(Some(v)),
        Err(_) if is_empty => Ok(None),
        Err(err) => Err(
            ErrorInner::EnvParseError {
                field: field.to_owned(),
                key: key.to_owned(),
                err: Box::new(err),
            }.into()
        ),
    }
}

pub fn from_env_with_deserializer<T>(
    key: &str,
    field: &str,
    deserialize: fn(crate::env::Deserializer) -> Result<T, crate::env::DeError>,
) -> Result<Option<T>, Error> {
    let s = get_env_var!(key, field);
    let is_empty = s.is_empty();

    match deserialize(crate::env::Deserializer::new(s)) {
        Ok(v) => Ok(Some(v)),
        Err(_) if is_empty => Ok(None),
        Err(e) => Err(ErrorInner::EnvDeserialization {
            key: key.into(),
            field: field.into(),
            msg: e.0,
        }.into()),
    }
}

/// `serde` does not implement `IntoDeserializer` for fixed size arrays. This
/// helper type is just used for this purpose.
pub struct ArrayIntoDeserializer<T, const N: usize>(pub [T; N]);

impl<'de, T, E, const N: usize> serde::de::IntoDeserializer<'de, E> for ArrayIntoDeserializer<T, N>
where
    T: serde::de::IntoDeserializer<'de, E>,
    E: serde::de::Error,
{
    type Deserializer = serde::de::value::SeqDeserializer<std::array::IntoIter<T, N>, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        serde::de::value::SeqDeserializer::new(self.0.into_iter())
    }
}

/// `serde` does implement `IntoDeserializer` for `HashMap` and `BTreeMap` but
/// we want to keep the exact source code order of entries, so we need our own
/// type.
pub struct MapIntoDeserializer<K, V>(pub Vec<(K, V)>);

impl<'de, K, V, E> serde::de::IntoDeserializer<'de, E> for MapIntoDeserializer<K, V>
where
    K: serde::de::IntoDeserializer<'de, E>,
    V: serde::de::IntoDeserializer<'de, E>,
    E: serde::de::Error,
{
    type Deserializer = serde::de::value::MapDeserializer<'de, std::vec::IntoIter<(K, V)>, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        serde::de::value::MapDeserializer::new(self.0.into_iter())
    }
}
