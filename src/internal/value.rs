//! A dynamically typed value.

use std::fmt;

use serde::de::{
    self,
    value::{Error, MapDeserializer, SeqDeserializer},
    Deserialize, Deserializer, IntoDeserializer, Visitor,
};


/// A dynamically typed value, used for arrays and maps in default values and
/// in `layer!`. Thanks to this, their items can have different types.
pub enum Value {
    Unit,
    Bool(bool),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    Seq(Vec<Value>),
    Map(Vec<(Value, Value)>),

    /// The original value could not be converted to `Value`. This error is
    /// returned when this is deserialized.
    Err(Error),
}

impl Value {
    pub fn new<'de>(v: impl IntoDeserializer<'de, Error>) -> Self {
        Self::deserialize(v.into_deserializer()).unwrap_or_else(Self::Err)
    }
}

macro_rules! visit {
    ($($method:ident: $ty:ty => $variant:ident,)*) => {
        $(
            fn $method<E>(self, v: $ty) -> Result<Value, E> {
                Ok(Value::$variant(v.into()))
            }
        )*
    };
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any value")
            }

            visit! {
                visit_bool: bool => Bool,
                visit_i64: i64 => I64,
                visit_u64: u64 => U64,
                visit_i128: i128 => I128,
                visit_u128: u128 => U128,
                visit_f64: f64 => F64,
                visit_str: &str => Str,
                visit_bytes: &[u8] => Bytes,
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Unit)
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Unit)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
                Value::deserialize(d)
            }

            fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
                Value::deserialize(d)
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut out = Vec::new();
                while let Some(item) = seq.next_element()? {
                    out.push(item);
                }
                Ok(Value::Seq(out))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    out.push(entry);
                }
                Ok(Value::Map(out))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Self::Unit => visitor.visit_unit(),
            Self::Bool(v) => visitor.visit_bool(v),
            Self::I64(v) => visitor.visit_i64(v),
            Self::U64(v) => visitor.visit_u64(v),
            Self::I128(v) => visitor.visit_i128(v),
            Self::U128(v) => visitor.visit_u128(v),
            Self::F64(v) => visitor.visit_f64(v),
            Self::Str(v) => visitor.visit_string(v),
            Self::Bytes(v) => visitor.visit_byte_buf(v),
            Self::Seq(v) => SeqDeserializer::new(v.into_iter()).deserialize_any(visitor),
            Self::Map(v) => MapDeserializer::new(v.into_iter()).deserialize_any(visitor),
            Self::Err(e) => Err(e),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Self::Unit => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self {
            Self::Str(v) => v.into_deserializer().deserialize_enum(name, variants, visitor),
            _ => self.deserialize_any(visitor),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}
