use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, ext::IdentExt};

use crate::ir::{self, Expr, FieldKind, FloatType, IntType, LeafKind, MapKey};



/// Generates the whole `const META: ... = ...;` item.
pub(super) fn gen(input: &ir::Input) -> TokenStream {
    fn env_tokens(env: &Option<String>) -> TokenStream {
        match env {
            Some(key) => quote! { std::option::Option::Some(#key) },
            None => quote! { std::option::Option::None },
        }
    }

    let name_str = input.name.unraw().to_string();
    let doc = &input.doc;
    let meta_fields = input.fields.iter().map(|f| {
        let name = f.name.unraw().to_string();
        let doc =  &f.doc;
        let kind = match &f.kind {
            FieldKind::Nested { ty } => {
                quote! {
                    confique::meta::FieldKind::Nested { meta: &<#ty as confique::Config>::META }
                }
            }
            FieldKind::Leaf { env, kind: LeafKind::Optional { .. }, ..} => {
                let env = env_tokens(env);
                quote! {
                    confique::meta::FieldKind::Leaf {
                        env: #env,
                        kind: confique::meta::LeafKind::Optional,
                    }
                }
            }
            FieldKind::Leaf { env, kind: LeafKind::Required { default, .. }, ..} => {
                let env = env_tokens(env);
                let default_value = match default {
                    Some(default) => {
                        let meta = default_value_to_meta_expr(default);
                        quote! { std::option::Option::Some(#meta) }
                    },
                    None => quote! { std::option::Option::None },
                };
                quote! {
                    confique::meta::FieldKind::Leaf {
                        env: #env,
                        kind: confique::meta::LeafKind::Required {
                            default: #default_value,
                        },
                    }
                }
            }
        };

        quote! {
            confique::meta::Field {
                name: #name,
                doc: &[ #(#doc),* ],
                kind: #kind,
            }
        }
    });

    quote! {
        const META: confique::meta::Meta = confique::meta::Meta {
            name: #name_str,
            doc: &[ #(#doc),* ],
            fields: &[ #( #meta_fields ),* ],
        };
    }
}

/// Helper macro to deduplicate logic for literals. Only used in the function
/// below.
macro_rules! match_literals {
    ($v:expr, $ns:ident, { $($other_arms:tt)* }) => {
        match $v {
            $ns::Bool(v) => quote! { confique::meta::$ns::Bool(#v) },
            $ns::Str(s) => quote! { confique::meta::$ns::Str(#s) },
            // Without known type, we use the same fallback as Rust.
            $ns::Int(i, ty) => {
                let variant = ty.unwrap_or(IntType::I32).variant();
                quote! { confique::meta::$ns::Integer(confique::meta::Integer::#variant(#i)) }
            }
            $ns::Float(f, ty) => {
                let variant = ty.unwrap_or(FloatType::F64).variant();
                quote! { confique::meta::$ns::Float(confique::meta::Float::#variant(#f)) }
            }
            $($other_arms)*
        }
    };
}

/// Generates the meta expression of type `meta::Expr` to be used for the
/// `default` field.
fn default_value_to_meta_expr(default: &Expr) -> TokenStream {
    match_literals!(default, Expr, {
        Expr::Array(items) => {
            let items = items.iter().map(default_value_to_meta_expr);
            quote! { confique::meta::Expr::Array(&[#( #items ),*]) }
        }
        Expr::Map(entries) => {
            let pairs = entries.iter().map(|e| {
                let key = match_literals!(&e.key, MapKey, {});
                let value = default_value_to_meta_expr(&e.value);
                quote! { confique::meta::MapEntry { key: #key, value: #value } }
            });
            quote! { confique::meta::Expr::Map(&[#( #pairs ),*]) }
        }
        Expr::Rust(_) => unreachable!("bug: Rust expression in default value"),
    })
}

impl IntType {
    /// Returns the name of the corresponding `meta::Integer` variant.
    fn variant(self) -> Ident {
        let name = match self {
            Self::U8 => "U8",
            Self::U16 => "U16",
            Self::U32 => "U32",
            Self::U64 => "U64",
            Self::U128 => "U128",
            Self::Usize => "Usize",
            Self::I8 => "I8",
            Self::I16 => "I16",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::I128 => "I128",
            Self::Isize => "Isize",
        };
        Ident::new(name, Span::call_site())
    }
}

impl FloatType {
    /// Returns the name of the corresponding `meta::Float` variant.
    fn variant(self) -> Ident {
        let name = match self {
            Self::F32 => "F32",
            Self::F64 => "F64",
        };
        Ident::new(name, Span::call_site())
    }
}
