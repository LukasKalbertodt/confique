//! Deserialize values from environment variables.

use std::fmt;

use serde::de::{Error as _, IntoDeserializer};


pub(crate) fn deserialize<'de, T: serde::Deserialize<'de>>(
    value: Option<String>,
) -> Result<T, DeError> {
    let mut deserializer = Deserializer { value };
    T::deserialize(&mut deserializer)
}


/// Private error type only for deserialization. Gets converted into
/// `ErrorKind::EnvDeserialization` before reaching the public API.
#[derive(PartialEq)]
pub(crate) struct DeError(pub(crate) String);

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


/// Deserializer type.
struct Deserializer {
    value: Option<String>,
}

impl Deserializer {
    fn need_value(&mut self) -> Result<String, DeError> {
        self.value.take().ok_or_else(|| DeError::custom("environment variable not set"))
    }
}

macro_rules! deserialize_via_parse {
    ($method:ident, $visit_method:ident, $int:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>
        {
            let s = self.need_value()?;
            let v = s.trim().parse().map_err(|e| {
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

impl<'de> serde::Deserializer<'de> for &mut Deserializer {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        match self.value.take() {
            None => visitor.visit_none(),
            Some(s) => s.into_deserializer().deserialize_any(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        let v = match self.need_value()?.trim() {
            "1" | "true" | "TRUE" => true,
            "0" | "false" | "FALSE" => false,
            other => return Err(DeError(format!("invalid value for bool: '{}'", other))),
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

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>
    {
        match self.value {
            None => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
        }
    }

    serde::forward_to_deserialize_any! {
        char str string
        bytes byte_buf
        unit unit_struct
        map
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

    fn de<'de, T: serde::Deserialize<'de>>(v: impl Into<Option<&'static str>>) -> Result<T, DeError> {
        deserialize(v.into().map(|s| s.to_owned()))
    }


    #[test]
    fn boolean() {
        assert_eq!(de("1"), Ok(Some(true)));
        assert_eq!(de("true "), Ok(Some(true)));
        assert_eq!(de("  TRUE"), Ok(Some(true)));
        assert_eq!(de("0  "), Ok(Some(false)));
        assert_eq!(de(" false"), Ok(Some(false)));
        assert_eq!(de("FALSE "), Ok(Some(false)));

        assert_eq!(de(None), Ok(Option::<bool>::None));
    }

    #[test]
    fn ints() {
        assert_eq!(de("0"), Ok(Some(0u8)));
        assert_eq!(de("-1 "), Ok(Some(-1i8)));
        assert_eq!(de(" 27"), Ok(Some(27u16)));
        assert_eq!(de("-27"), Ok(Some(-27i16)));
        assert_eq!(de("   4301"), Ok(Some(4301u32)));
        assert_eq!(de(" -123456"), Ok(Some(-123456i32)));
        assert_eq!(de(" 986543210    "), Ok(Some(986543210u64)));
        assert_eq!(de("-986543210"), Ok(Some(-986543210i64)));

        assert_eq!(de(None), Ok(Option::<i8>::None));
        assert_eq!(de(None), Ok(Option::<u8>::None));
        assert_eq!(de(None), Ok(Option::<i16>::None));
        assert_eq!(de(None), Ok(Option::<u16>::None));
        assert_eq!(de(None), Ok(Option::<i32>::None));
        assert_eq!(de(None), Ok(Option::<u32>::None));
        assert_eq!(de(None), Ok(Option::<i64>::None));
        assert_eq!(de(None), Ok(Option::<u64>::None));
    }

    #[test]
    fn floats() {
        assert_eq!(de("3.1415"), Ok(Some(3.1415f32)));
        assert_eq!(de("-123.456"), Ok(Some(-123.456f64)));

        assert_eq!(de(None), Ok(Option::<f32>::None));
        assert_eq!(de(None), Ok(Option::<f64>::None));
    }
}
