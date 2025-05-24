use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::ir::{self, Expr, FieldKind, LeafKind, MapKey};



/// Generates the whole `const META: ... = ...;` item.
pub(super) fn gen(input: &ir::Input) -> TokenStream {
    fn env_tokens(env: &Option<String>) -> TokenStream {
        match env {
            Some(key) => quote! { std::option::Option::Some(#key) },
            None => quote! { std::option::Option::None },
        }
    }

    let name_str = input.name.to_string();
    let doc = &input.doc;
    let meta_fields = input.fields.iter().map(|f| {
        let name = f.name.to_string();
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
            FieldKind::Leaf { env, kind: LeafKind::Required { default, ty, .. }, ..} => {
                let env = env_tokens(env);
                let default_value = match default {
                    Some(default) => {
                        let meta = default_value_to_meta_expr(default, Some(&ty));
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
    ($v:expr, $ty:expr, $ns:ident, { $($other_arms:tt)* }) => {
        match $v {
            $ns::Bool(v) => quote! { confique::meta::$ns::Bool(#v) },
            $ns::Str(s) => quote! { confique::meta::$ns::Str(#s) },
            $ns::Int(i) => {
                let variant = infer_type(i.suffix(), $ty, "I32", int_type_to_variant);
                quote! { confique::meta::$ns::Integer(confique::meta::Integer::#variant(#i)) }
            }
            $ns::Float(f) => {
                let variant = infer_type(f.suffix(), $ty, "F64", float_type_to_variant);
                quote! { confique::meta::$ns::Float(confique::meta::Float::#variant(#f)) }
            }
            $($other_arms)*
        }
    };
}

/// Generates the meta expression of type `meta::Expr` to be used for the
/// `default` field. `ty` is the type of the field that is used to better infer
/// the exact type of the default value.
fn default_value_to_meta_expr(default: &Expr, ty: Option<&syn::Type>) -> TokenStream {
    match_literals!(default, ty, Expr, {
        Expr::Array(items) => {
            let item_type = ty.and_then(get_array_item_type);
            let items = items.iter().map(|item| default_value_to_meta_expr(item, item_type));
            quote! { confique::meta::Expr::Array(&[#( #items ),*]) }
        }
        Expr::Map(entries) => {
            // TODO: use `Option::unzip` once stable
            let types = ty.and_then(get_map_entry_types);
            let key_type = types.map(|(t, _)| t);
            let value_type = types.map(|(_, v)| v);

            let pairs = entries.iter().map(|e| {
                let key = match_literals!(&e.key, key_type, MapKey, {});
                let value = default_value_to_meta_expr(&e.value, value_type);
                quote! { confique::meta::MapEntry { key: #key, value: #value } }
            });
            quote! { confique::meta::Expr::Map(&[#( #pairs ),*]) }
        }
    })
}

/// Maps an integer type to the `meta::Expr` variant (e.g. `u32` -> `U32`).
fn int_type_to_variant(suffix: &str) -> Option<&'static str> {
    match suffix {
        "u8" => Some("U8"),
        "u16" => Some("U16"),
        "u32" => Some("U32"),
        "u64" => Some("U64"),
        "u128" => Some("U128"),
        "usize" => Some("Usize"),
        "i8" => Some("I8"),
        "i16" => Some("I16"),
        "i32" => Some("I32"),
        "i64" => Some("I64"),
        "i128" => Some("I128"),
        "isize" => Some("Isize"),
        _ => None,
    }
}

/// Maps a float type to the `meta::Expr` variant (e.g. `f32` -> `F32`).
fn float_type_to_variant(suffix: &str) -> Option<&'static str> {
    match suffix {
        "f32" => Some("F32"),
        "f64" => Some("F64"),
        _ => None,
    }
}

/// Tries to infer the type of an int or float default value.
///
/// To figure out the type of int or float literals, we first look at the type
/// suffix of the literal. If it is specified, we use that. Otherwise we check
/// if the field type is a known float/integer type. If so, we use that.
/// Otherwise we use a default.
fn infer_type(
    suffix: &str,
    field_ty: Option<&syn::Type>,
    default: &str,
    map: fn(&str) -> Option<&'static str>,
) -> Ident {
    let variant = map(suffix)
        .or_else(|| {
            if let Some(syn::Type::Path(syn::TypePath { qself: None, path })) = field_ty {
                path.get_ident().and_then(|i| map(&i.to_string()))
            } else {
                None
            }
        })
        .unwrap_or(default);

    Ident::new(variant, Span::call_site())
}

/// Tries to extract the type of the item of a field with an array default
/// value. Examples: `&[u32]` -> `u32`, `Vec<String>` -> `String`.
fn get_array_item_type(ty: &syn::Type) -> Option<&syn::Type> {
    match ty {
        // The easy types.
        syn::Type::Slice(slice) => Some(&slice.elem),
        syn::Type::Array(array) => Some(&*array.elem),

        // This is the least clear case. We certainly want to cover `Vec<T>` but
        // ideally some more cases. On the other hand, we just can't really
        // know, so some incorrect guesses are definitely expected here. Most
        // are likely filtered out by applying `gen_meta_default` to it, but
        // some will result in a wrong default value type. But people can
        // always just add a prefix to the literal in those cases.
        //
        // We simply check if the last element in the path has exactly one
        // generic type argument, in which case we use that.
        syn::Type::Path(p) => {
            let args = match &p.path.segments.last().expect("empty type path").arguments {
                syn::PathArguments::AngleBracketed(args) => &args.args,
                _ => return None,
            };

            if args.len() != 1 {
                return None;
            }

            match &args[0] {
                syn::GenericArgument::Type(t) => Some(t),
                _ => None,
            }
        },

        // Just recurse on inner type.
        syn::Type::Reference(r) => get_array_item_type(&r.elem),
        syn::Type::Group(g) => get_array_item_type(&g.elem),
        syn::Type::Paren(p) => get_array_item_type(&p.elem),

        _ => None,
    }
}

/// Tries to extract the key and value types from a map value. Examples:
/// `HashMap<String, u32>` -> `(String, u32)`.
fn get_map_entry_types(ty: &syn::Type) -> Option<(&syn::Type, &syn::Type)> {
    match ty {
        // We simply check if the last element in the path has exactly two
        // generic type arguments, in which case we use those. Otherwise we
        // can't really know.
        syn::Type::Path(p) => {
            let args = match &p.path.segments.last().expect("empty type path").arguments {
                syn::PathArguments::AngleBracketed(args) => &args.args,
                _ => return None,
            };

            if args.len() != 2 {
                return None;
            }

            match (&args[0], &args[1]) {
                (syn::GenericArgument::Type(k), syn::GenericArgument::Type(v)) => Some((k, v)),
                _ => None,
            }
        },

        // Just recurse on inner type.
        syn::Type::Group(g) => get_map_entry_types(&g.elem),
        syn::Type::Paren(p) => get_map_entry_types(&p.elem),

        _ => None,
    }
}
