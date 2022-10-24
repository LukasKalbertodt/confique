use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, spanned::Spanned};

use crate::ir::{self, FieldKind, LeafKind};

mod meta;


/// The main function to generate the output token stream from the parse IR.
pub(crate) fn gen(input: ir::Input) -> TokenStream {
    let partial_mod = gen_partial_mod(&input);
    let config_impl = gen_config_impl(&input);

    quote! {
        #config_impl
        #partial_mod
    }
}

/// Generates the `impl Config for ... { ... }`.
fn gen_config_impl(input: &ir::Input) -> TokenStream {
    let name = &input.name;
    let (partial_mod_name, partial_struct_name) = partial_names(&input.name);

    let field_names = input.fields.iter().map(|f| &f.name);
    let from_exprs = input.fields.iter().map(|f| {
        let field_name = &f.name;
        let path = field_name.to_string();
        match f.kind {
            FieldKind::Nested { .. } => {
                quote! {
                    confique::internal::map_err_prefix_path(
                        confique::Config::from_partial(partial.#field_name),
                        #path,
                    )?
                }
            }
            FieldKind::Leaf { kind: LeafKind::Optional { .. }, .. } => {
                quote! { partial.#field_name }
            }
            FieldKind::Leaf { kind: LeafKind::Required { .. }, .. } => {
                quote! {
                    confique::internal::unwrap_or_missing_value_err(partial.#field_name, #path)?
                }
            }
        }
    });


    let meta_item = meta::gen(input);
    quote! {
        #[automatically_derived]
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

/// Generates the whole `mod ... { ... }` that defines the partial type and
/// related items.
fn gen_partial_mod(input: &ir::Input) -> TokenStream {
    let (mod_name, struct_name) = partial_names(&input.name);
    let visibility = &input.visibility;

    fn deserialize_fn_name(field_name: &Ident) -> Ident {
        // This has an ugly name to avoid clashing with imported names.
        quote::format_ident!("__confique_deserialize_{field_name}")
    }

    // Prepare some tokens per field.
    let field_names = input.fields.iter().map(|f| &f.name).collect::<Vec<_>>();
    let struct_fields = input.fields.iter().map(|f| {
        let name = &f.name;

        // We have to use the span of the field's name here so that error
        // messages from the `derive(serde::Deserialize)` have the correct span.
        let inner_vis = inner_visibility(&input.visibility, name.span());
        match &f.kind {
            FieldKind::Leaf { kind, deserialize_with, .. } => {
                let ty = kind.inner_ty();
                let attr = match deserialize_with {
                    None => quote! {},
                    Some(p) => {
                        let fn_name = deserialize_fn_name(&f.name).to_string();
                        quote_spanned! {p.span()=>
                            #[serde(default, deserialize_with = #fn_name)]
                        }
                    }
                };

                let main = quote_spanned! {name.span()=> #inner_vis #name: Option<#ty> };
                quote! { #attr #main }
            }
            FieldKind::Nested { ty } => {
                let ty_span = ty.span();
                let field_ty = quote_spanned! {ty_span=> <#ty as confique::Config>::Partial };
                quote! {
                    #[serde(default = "confique::Partial::empty")]
                    #inner_vis #name: #field_ty
                }
            },
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
            FieldKind::Leaf {
                kind: LeafKind::Required { default: Some(default), .. },
                deserialize_with,
                ..
            } => {
                let msg = format!(
                    "default config value for `{}::{}` cannot be deserialized",
                    input.name,
                    f.name,
                );
                let expr = default_value_to_deserializable_expr(&default);

                match deserialize_with {
                    None => quote! {
                        Some(confique::internal::deserialize_default(#expr).expect(#msg))
                    },
                    Some(p) => quote! {
                        Some(#p(confique::internal::into_deserializer(#expr)).expect(#msg))
                    },
                }
            }
            FieldKind::Leaf { .. } => quote! { None },
            FieldKind::Nested { .. } => quote! { confique::Partial::default_values() },
        }
    });

    let from_env_fields = input.fields.iter().map(|f| match &f.kind {
        FieldKind::Leaf {
            env: Some(key),
            deserialize_with,
            parse_env,
            ..
        } => {
            let field = format!("{}::{}", input.name, f.name);
            match (parse_env, deserialize_with) {
                (None, None) => quote! {
                    confique::internal::from_env(#key, #field)?
                },
                (None, Some(deserialize_with)) => quote! {
                    confique::internal::deserialize_from_env_with(#key, #field, #deserialize_with)?
                },
                (Some(parse_env), None) | (Some(parse_env), Some(_)) => quote! {
                    confique::internal::parse_from_env_with(#key, #field, #parse_env)?
                },
            }
        }
        FieldKind::Leaf { .. } => quote! { None },
        FieldKind::Nested { .. } => quote! { confique::Partial::from_env()? },
    });

    let fallbacks = input.fields.iter().map(|f| {
        let name = &f.name;
        if f.is_leaf() {
            quote! { self.#name.or(fallback.#name) }
        } else {
            quote! { self.#name.with_fallback(fallback.#name) }
        }
    });

    let is_empty_exprs = input.fields.iter().map(|f| {
        let name = &f.name;
        if f.is_leaf() {
            quote! { self.#name.is_none() }
        } else {
            quote! { self.#name.is_empty() }
        }
    });

    let is_complete_expr = input.fields.iter().map(|f| {
        let name = &f.name;
        match &f.kind {
            FieldKind::Leaf { kind, .. } => {
                if kind.is_required() {
                    quote! { self.#name.is_some() }
                } else {
                    quote! { true }
                }
            }
            FieldKind::Nested { .. } => quote! { self.#name.is_complete() },
        }
    });

    let deserialize_fns = input.fields.iter().filter_map(|f| {
        match &f.kind {
            FieldKind::Leaf { kind, deserialize_with: Some(p), .. } => {
                let fn_name = deserialize_fn_name(&f.name);
                let ty = kind.inner_ty();

                Some(quote! {
                    fn #fn_name<'de, D>(deserializer: D) -> Result<Option<#ty>, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        #p(deserializer).map(Some)
                    }
                })
            }
            _ => None,
        }
    });

    let nested_bounds = input.fields.iter().filter_map(|f| {
        match &f.kind {
            FieldKind::Nested { ty } => Some(quote! { #ty: confique::Config }),
            FieldKind::Leaf { .. } => None,
        }
    });

    let struct_visibility = inner_visibility(&input.visibility, Span::call_site());
    quote! {
        #visibility mod #mod_name {
            use super::*;

            #[derive(confique::serde::Deserialize)]
            #struct_visibility struct #struct_name {
                #( #struct_fields, )*
            }

            #[automatically_derived]
            impl confique::Partial for #struct_name where #( #nested_bounds, )* {
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

                fn from_env() -> Result<Self, confique::Error> {
                    Ok(Self {
                        #( #field_names: #from_env_fields, )*
                    })
                }

                fn with_fallback(self, fallback: Self) -> Self {
                    Self {
                        #( #field_names: #fallbacks, )*
                    }
                }

                fn is_empty(&self) -> bool {
                    true #(&& #is_empty_exprs)*
                }

                fn is_complete(&self) -> bool {
                    true #(&& #is_complete_expr)*
                }
            }

            #(#deserialize_fns)*
        }
    }
}

/// Returns the names of the module and struct for the partial type:
/// `(mod_name, struct_name)`.
fn partial_names(original_name: &Ident) -> (Ident, Ident) {
    use heck::SnakeCase;
    (
        format_ident!("confique_partial_{}", original_name.to_string().to_snake_case()),
        format_ident!("Partial{original_name}"),
    )
}

/// Generates a Rust expression from the default value that implemenets
/// `serde::de::IntoDeserializer`.
fn default_value_to_deserializable_expr(expr: &ir::Expr) -> TokenStream {
    match expr {
        ir::Expr::Str(lit) => quote! { #lit },
        ir::Expr::Int(lit) => quote! { #lit },
        ir::Expr::Float(lit) => quote! { #lit },
        ir::Expr::Bool(lit) => quote! { #lit },
        ir::Expr::Array(arr) => {
            let items = arr.iter().map(default_value_to_deserializable_expr);

            // Empty arrays cause "cannot infer type" errors here. However, it
            // really doesn't matter what type the array has as there are 0
            // elements anyway. So we just pick `()`.
            let type_annotation = if arr.is_empty() {
                quote! { as [(); 0] }
            } else {
                quote! {}
            };
            quote! { confique::internal::ArrayIntoDeserializer([ #(#items),* ] #type_annotation) }
        },
        ir::Expr::Map(entries) => {
            let items = entries.iter().map(|e| {
                let key = default_value_to_deserializable_expr(&e.key.clone().into());
                let value = default_value_to_deserializable_expr(&e.value);
                quote! { (#key, #value) }
            });

            // Empty arrays cause "cannot infer type" errors here. However, it
            // really doesn't matter what type the array has as there are 0
            // elements anyway. So we just pick `()`.
            let type_annotation = if entries.is_empty() {
                quote! { as Vec<((), ())> }
            } else {
                quote! {}
            };
            quote! { confique::internal::MapIntoDeserializer(vec![ #(#items),* ] #type_annotation) }
        },
    }
}

/// Returns tokens defining the visibility of the items in the inner module.
fn inner_visibility(outer: &syn::Visibility, span: Span) -> TokenStream {
    match outer {
        // These visibilities can be used as they are. No adjustment needed.
        syn::Visibility::Public(_) | syn::Visibility::Crate(_) => quote_spanned! {span=> #outer },

        // The inherited one is relative to the parent module.
        syn::Visibility::Inherited => quote_spanned! {span=> pub(super) },

        // For `pub(crate)`
        syn::Visibility::Restricted(r) if r.path.is_ident("crate") && r.in_token.is_none() => {
            quote_spanned! {span=> pub(crate) }
        },

        // If the path in the `pub(in <path>)` visibility is absolute, we can
        // use it like that as well.
        syn::Visibility::Restricted(r) if r.path.leading_colon.is_some() => {
            quote_spanned! {span=> #outer }
        },

        // But in the case `pub(in <path>)` with a relative path, we have to
        // prefix `super::`.
        syn::Visibility::Restricted(r) => {
            let path = &r.path;
            quote_spanned! {span=> pub(in super::#path) }
        }
    }
}
