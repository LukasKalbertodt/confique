use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::Ident;

use crate::ir::{self, Expr, FieldKind};


pub(crate) fn gen(input: ir::Input) -> TokenStream {
    let partial_mod = gen_partial_mod(&input);
    let config_impl = gen_config_impl(&input);

    quote! {
        #config_impl
        #partial_mod
    }
}

fn gen_config_impl(input: &ir::Input) -> TokenStream {
    let name = &input.name;
    let (partial_mod_name, partial_struct_name) = partial_names(&input.name);

    let field_names = input.fields.iter().map(|f| &f.name);
    let from_exprs = input.fields.iter().map(|f| {
        let field_name = &f.name;
        let path = field_name.to_string();
        if !f.is_leaf() {
            quote! {
                confique::Config::from_partial(partial.#field_name).map_err(|e| {
                    confique::internal::prepend_missing_value_error(e, #path)
                })?
            }
        } else if unwrap_option(&f.ty).is_none() {
            quote! {
                partial.#field_name.ok_or(confique::internal::missing_value_error(#path.into()))?
            }
        } else {
            quote! { partial.#field_name }
        }
    });


    let meta_item = gen_meta(input);
    quote! {
        impl confique::Config for #name {
            type Partial = #partial_mod_name::#partial_struct_name;

            fn from_partial(partial: Self::Partial) -> Result<Self, confique::Error> {
                Ok(Self {
                    #( #field_names: #from_exprs, )*
                })
            }

            #meta_item
        }
    }
}


/// Returns the names of the module and struct for the partial type:
/// `(mod_name, struct_name)`.
fn partial_names(original_name: &Ident) -> (Ident, Ident) {
    use heck::SnakeCase;
    (
        format_ident!("confique_partial_{}", original_name.to_string().to_snake_case()),
        format_ident!("Partial{}", original_name),
    )
}

fn gen_partial_mod(input: &ir::Input) -> TokenStream {
    let (mod_name, struct_name) = partial_names(&input.name);
    let visibility = &input.visibility;
    let inner_visibility = inner_visibility(&input.visibility);

    // Prepare some tokens per field.
    let field_names = input.fields.iter().map(|f| &f.name).collect::<Vec<_>>();
    let struct_fields = input.fields.iter().map(|f| {
        let name = &f.name;
        if f.is_leaf() {
            let inner = unwrap_option(&f.ty).unwrap_or(&f.ty);
            quote! { #inner_visibility #name: Option<#inner>, }
        } else {
            let ty = &f.ty;
            quote! {
                #[serde(default = "confique::Partial::empty")]
                #inner_visibility #name: <#ty as confique::Config>::Partial,
            }
        }
    });
    let empty_values = input.fields.iter().map(|f| {
        if f.is_leaf() {
            quote! { None }
        } else {
            quote! { confique::Partial::empty() }
        }
    });
    let defaults = input.fields.iter().map(|f| {
        match &f.kind {
            FieldKind::Leaf { default: None } => quote! { None },
            FieldKind::Leaf { default: Some(default) } => {
                let msg = format!(
                    "default config value for `{}::{}` cannot be deserialized",
                    input.name,
                    f.name,
                );

                quote! {
                    Some(confique::internal::deserialize_default(#default).expect(#msg))
                }
            }
            FieldKind::Nested => {
                if unwrap_option(&f.ty).is_some() {
                    quote! { Some(confique::Partial::default_values()) }
                } else {
                    quote! { confique::Partial::default_values() }
                }
            }
        }
    });
    let fallbacks= input.fields.iter().map(|f| {
        let name = &f.name;
        if f.is_leaf() {
            quote! { self.#name.or(fallback.#name) }
        } else {
            quote! { self.#name.with_fallback(fallback.#name) }
        }
    });

    quote! {
        #visibility mod #mod_name {
            use super::*;

            #[derive(confique::serde::Deserialize)]
            #inner_visibility struct #struct_name {
                #( #struct_fields )*
            }

            impl confique::Partial for #struct_name {
                fn empty() -> Self {
                    Self {
                        #( #field_names: #empty_values, )*
                    }
                }

                fn default_values() -> Self {
                    Self {
                        #( #field_names: #defaults, )*
                    }
                }

                fn with_fallback(self, fallback: Self) -> Self {
                    Self {
                        #( #field_names: #fallbacks, )*
                    }
                }
            }
        }
    }
}


/// Generates the whole `const META` item.
fn gen_meta(input: &ir::Input) -> TokenStream {
    let name_str = input.name.to_string();
    let doc = &input.doc;
    let meta_fields = input.fields.iter().map(|f| {
        let name = f.name.to_string();
        let doc =  &f.doc;
        let kind = match &f.kind {
            FieldKind::Nested => {
                let ty = &f.ty;
                quote! {
                    confique::meta::FieldKind::Nested { meta: &<#ty as confique::Config>::META }
                }
            }
            FieldKind::Leaf { default } => {
                let default_value = gen_meta_default(default, &f.ty);
                quote! {
                    confique::meta::FieldKind::Leaf { default: #default_value }
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

/// Generates the meta expression of type `meta::Expr` to be used for the
/// `default` field.
fn gen_meta_default(default: &Option<Expr>, ty: &syn::Type) -> TokenStream {
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

    fn float_type_to_variant(suffix: &str) -> Option<&'static str> {
        match suffix {
            "f32" => Some("F32"),
            "f64" => Some("F64"),
            _ => None,
        }
    }

    // To figure out the type of int or float literals, we first look at the
    // type suffix of the literal. If it is specified, we use that. Otherwise
    // we check if the field type is a known float/integer type. If so, we use
    // that. Otherwise we use a default.
    fn infer_type(
        suffix: &str,
        field_ty: &syn::Type,
        default: &str,
        map: fn(&str) -> Option<&'static str>,
    ) -> Ident {
        let variant = int_type_to_variant(suffix)
            .or_else(|| {
                if let syn::Type::Path(syn::TypePath { qself: None, path }) = field_ty {
                    path.get_ident().and_then(|i| map(&i.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or(default);

        Ident::new(variant, Span::call_site())
    }


    if let Some(default) = default {
        let v = match default {
            Expr::Bool(v) => quote! { confique::meta::Expr::Bool(#v) },
            Expr::Str(s) => quote! { confique::meta::Expr::Str(#s) },
            Expr::Int(i) => {
                let variant = infer_type(i.suffix(), ty, "I32", int_type_to_variant);
                quote! { confique::meta::Expr::Integer(confique::meta::Integer::#variant(#i)) }
            }
            Expr::Float(f) => {
                let variant = infer_type(f.suffix(), ty, "F64", float_type_to_variant);
                quote! { confique::meta::Expr::Float(confique::meta::Float::#variant(#f)) }
            }
        };

        quote! { Some(#v) }
    } else {
        quote! { None }
    }
}

/// Checks if the given type is an `Option` and if so, return the inner type.
///
/// Note: this function clearly shows one of the major shortcomings of proc
/// macros right now: we do not have access to the compiler's type tables and
/// can only check if it "looks" like an `Option`. Of course, stuff can go
/// wrong. But that's the best we can do and it's highly unlikely that someone
/// shadows `Option`.
fn unwrap_option(ty: &syn::Type) -> Option<&syn::Type> {
    let ty = match ty {
        syn::Type::Path(path) => path,
        _ => return None,
    };

    if ty.qself.is_some() || ty.path.leading_colon.is_some() {
        return None;
    }

    let valid_paths = [
        &["Option"] as &[_],
        &["std", "option", "Option"],
        &["core", "option", "Option"],
    ];
    if !valid_paths.iter().any(|vp| ty.path.segments.iter().map(|s| &s.ident).eq(*vp)) {
        return None;
    }

    let args = match &ty.path.segments.last().unwrap().arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => return None,
    };

    if args.args.len() != 1 {
        return None;
    }

    match &args.args[0] {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}

impl ToTokens for ir::Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Str(lit) => lit.to_tokens(tokens),
            Self::Int(lit) => lit.to_tokens(tokens),
            Self::Float(lit) => lit.to_tokens(tokens),
            Self::Bool(lit) => lit.to_tokens(tokens),
        }
    }
}

fn inner_visibility(outer: &syn::Visibility) -> TokenStream {
    match outer {
        // These visibilities can be used as they are. No adjustment needed.
        syn::Visibility::Public(_) | syn::Visibility::Crate(_) => quote! { outer },

        // The inherited one is relative to the parent module.
        syn::Visibility::Inherited => quote! { pub(super) },

        // If the path in the `pub(in <path>)` visibility is absolute, we can
        // use it like that as well.
        syn::Visibility::Restricted(r) if r.path.leading_colon.is_some() => quote! { outer },

        // But in the case `pub(in <path>)` with a relative path, we have to
        // prefix `super::`.
        syn::Visibility::Restricted(r) => {
            let path = &r.path;
            quote! { pub(in super::#path) }
        }
    }
}
