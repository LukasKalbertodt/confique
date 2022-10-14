//! Deserialize values from environment variables.

use std::fmt;

use serde::de::IntoDeserializer;


/// Error type only for deserialization of env values.
///
/// Semantically private, only public as it's used in the API of the `internal`
/// module. Gets converted into `ErrorKind::EnvDeserialization` before reaching
/// the real public API.
#[derive(PartialEq)]
pub struct DeError(pub(crate) String);

impl std::error::Error for DeError {}

impl fmt::Debug for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl serde::de::Error for DeError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self(msg.to_string())
    }
}


/// Deserializer type. Semantically private (see `DeError`).
pub struct Deserializer {
    value: String,
}

impl Deserializer {
    pub(crate) fn new(value: String) -> Self {
        Self { value }
    }
}

macro_rules! deserialize_via_parse {
    ($method:ident, $visit_method:ident, $int:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>
        {
            let s = self.value.trim();
            let v = s.parse().map_err(|e| {
                DeError(format!(
                    concat!("invalid value '{}' for type ", stringify!($int), ": {}"),
                    s,
                    e,
                ))
            })?;
            visitor.$visit_method(v)
        }
    };
}

impl<'de> serde::Deserializer<'de> for Deserializer {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        self.value.into_deserializer().deserialize_any(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        let v = match self.value.trim() {
            "1" | "true" | "TRUE" => true,
            "0" | "false" | "FALSE" => false,
            other => return Err(DeError(format!("invalid value for bool: '{other}'"))),
        };

        visitor.visit_bool(v)
    }

    deserialize_via_parse!(deserialize_i8, visit_i8, i8);
    deserialize_via_parse!(deserialize_i16, visit_i16, i16);
    deserialize_via_parse!(deserialize_i32, visit_i32, i32);
    deserialize_via_parse!(deserialize_i64, visit_i64, i64);
    deserialize_via_parse!(deserialize_u8, visit_u8, u8);
    deserialize_via_parse!(deserialize_u16, visit_u16, u16);
    deserialize_via_parse!(deserialize_u32, visit_u32, u32);
    deserialize_via_parse!(deserialize_u64, visit_u64, u64);
    deserialize_via_parse!(deserialize_f32, visit_f32, f32);
    deserialize_via_parse!(deserialize_f64, visit_f64, f64);

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        char str string
        bytes byte_buf
        unit unit_struct
        map
        option
        struct
        identifier
        ignored_any

        // TODO: think about manually implementing these
        enum
        seq
        tuple tuple_struct
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn de<'de, T: serde::Deserialize<'de>>(v: &'static str) -> Result<T, DeError> {
        T::deserialize(Deserializer { value: v.into() })
    }


    #[test]
    fn boolean() {
        assert_eq!(de("1"), Ok(true));
        assert_eq!(de("true "), Ok(true));
        assert_eq!(de("  TRUE"), Ok(true));
        assert_eq!(de("0  "), Ok(false));
        assert_eq!(de(" false"), Ok(false));
        assert_eq!(de("FALSE "), Ok(false));
    }

    #[test]
    fn ints() {
        assert_eq!(de("0"), Ok(0u8));
        assert_eq!(de("-1 "), Ok(-1i8));
        assert_eq!(de(" 27"), Ok(27u16));
        assert_eq!(de("-27"), Ok(-27i16));
        assert_eq!(de("   4301"), Ok(4301u32));
        assert_eq!(de(" -123456"), Ok(-123456i32));
        assert_eq!(de(" 986543210    "), Ok(986543210u64));
        assert_eq!(de("-986543210"), Ok(-986543210i64));
    }

    #[test]
    fn floats() {
        assert_eq!(de("3.1415"), Ok(3.1415f32));
        assert_eq!(de("-123.456"), Ok(-123.456f64));
    }
}
