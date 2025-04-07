//! Definition of the intermediate representation.

use proc_macro2::TokenStream;


/// The parsed input to the `gen_config` macro.
pub(crate) struct Input {
    pub(crate) doc: Vec<String>,
    pub(crate) visibility: syn::Visibility,
    pub(crate) partial_attrs: Vec<TokenStream>,
    pub(crate) validate: Option<syn::Path>,
    pub(crate) name: syn::Ident,
    pub(crate) fields: Vec<Field>,
}

pub(crate) struct Field {
    pub(crate) doc: Vec<String>,
    pub(crate) name: syn::Ident,
    pub(crate) kind: FieldKind,

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
        validate: Option<FieldValidator>,
        partial_attr: Option<TokenStream>,
        kind: LeafKind,
    },

    /// A nested configuration. The type is never `Option<_>`.
    Nested {
        ty: syn::Type,
        partial_attr: Option<TokenStream>
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
pub(crate) enum Expr {
    Str(syn::LitStr),
    Int(syn::LitInt),
    Float(syn::LitFloat),
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
    Int(syn::LitInt),
    Float(syn::LitFloat),
    Bool(syn::LitBool),
}

impl From<MapKey> for Expr {
    fn from(src: MapKey) -> Self {
        match src {
            MapKey::Str(v) => Self::Str(v),
            MapKey::Int(v) => Self::Int(v),
            MapKey::Float(v) => Self::Float(v),
            MapKey::Bool(v) => Self::Bool(v),
        }
    }
}
