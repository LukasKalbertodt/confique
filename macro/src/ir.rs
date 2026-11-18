//! Definition of the intermediate representation.

use proc_macro2::TokenStream;


/// The parsed input to the `gen_config` macro.
pub(crate) struct Input {
    pub(crate) doc: Vec<String>,
    pub(crate) visibility: syn::Visibility,
    pub(crate) layer_attrs: Vec<TokenStream>,
    pub(crate) validate: Option<syn::Path>,
    pub(crate) name: syn::Ident,
    pub(crate) fields: Vec<Field>,
}

pub(crate) struct Field {
    pub(crate) doc: Vec<String>,
    pub(crate) name: syn::Ident,
    pub(crate) kind: FieldKind,
    pub(crate) layer_attrs: Vec<TokenStream>

    // TODO:
    // - serde attributes
    // - attributes
    // - example
}

pub(crate) enum FieldKind {
    Leaf {
        env: Option<String>,
        deserialize_with: Option<syn::Path>,
        parse_env: Option<syn::Path>,
        validate: Vec<FieldValidator>,
        kind: LeafKind,
    },

    /// A nested configuration. The type is never `Option<_>`.
    Nested {
        ty: syn::Type,
    },
}

pub(crate) enum FieldValidator {
    Fn(syn::Path),
    Simple(TokenStream, String),
}

pub(crate) enum LeafKind {
    /// A non-optional leaf. `ty` is not `Option<_>`.
    Required {
        default: Option<Expr>,
        ty: syn::Type,
    },

    /// A leaf with type `Option<_>`.
    Optional {
        inner_ty: syn::Type,
    },
}

impl LeafKind {
    pub(crate) fn is_required(&self) -> bool {
        matches!(self, Self::Required { .. })
    }

    pub(crate) fn inner_ty(&self) -> &syn::Type {
        match self {
            Self::Required { ty, .. } => ty,
            Self::Optional { inner_ty } => inner_ty,
        }
    }
}

/// The kinds of expressions (just literals) we allow for default or example
/// values.
///
/// The types of int and float literals are taken from their suffix or inferred
/// from the field type. `None` means neither worked, in which case Rust's
/// fallback (`i32` and `f64`) applies.
pub(crate) enum Expr {
    Str(syn::LitStr),
    Int(syn::LitInt, Option<IntType>),
    Float(syn::LitFloat, Option<FloatType>),
    Bool(syn::LitBool),
    Array(Vec<Expr>),
    Map(Vec<MapEntry>),
}

pub(crate) struct MapEntry {
    pub(crate) key: MapKey,
    pub(crate) value: Expr,
}

#[derive(Clone)]
pub(crate) enum MapKey {
    Str(syn::LitStr),
    Int(syn::LitInt, Option<IntType>),
    Float(syn::LitFloat, Option<FloatType>),
    Bool(syn::LitBool),
}

impl From<MapKey> for Expr {
    fn from(src: MapKey) -> Self {
        match src {
            MapKey::Str(v) => Self::Str(v),
            MapKey::Int(v, ty) => Self::Int(v, ty),
            MapKey::Float(v, ty) => Self::Float(v, ty),
            MapKey::Bool(v) => Self::Bool(v),
        }
    }
}

/// The type of an integer literal in a default value.
#[derive(Clone, Copy)]
pub(crate) enum IntType {
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
}

impl IntType {
    /// Parses a literal suffix or type name, e.g. `u64`.
    pub(crate) fn from_suffix(s: &str) -> Option<Self> {
        match s {
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            _ => None,
        }
    }
}

/// The type of a float literal in a default value.
#[derive(Clone, Copy)]
pub(crate) enum FloatType {
    F32,
    F64,
}

impl FloatType {
    /// Parses a literal suffix or type name, e.g. `f32`.
    pub(crate) fn from_suffix(s: &str) -> Option<Self> {
        match s {
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            _ => None,
        }
    }
}
