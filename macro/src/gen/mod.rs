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

    let validation = input.validate.as_ref().map(|v| quote! {
        confique::internal::do_validate_struct(&out, &#v)?;
    });

    let meta_item = meta::gen(input);
    quote! {
        #[automatically_derived]
        impl confique::Config for #name {
            type Partial = #partial_mod_name::#partial_struct_name;

            fn from_partial(partial: Self::Partial) -> std::result::Result<Self, confique::Error> {
                let out = Self {
                    #( #field_names: #from_exprs, )*
                };
                #validation
                std::result::Result::Ok(out)
            }

            #meta_item
        }
    }
}

/// Generates the whole `mod ... { ... }` that defines the partial type and
/// related items.
fn gen_partial_mod(input: &ir::Input) -> TokenStream {
    // Iterate through all fields, collecting field-relevant parts to be sliced
    // in the various methods.
    let mut parts = Parts::default();
    for f in &input.fields {
        gen_parts_for_field(f, input, &mut parts);
    }
    let Parts {
        field_names,
        struct_fields,
        nested_bounds,
        empty_exprs,
        default_exprs,
        from_env_exprs,
        fallback_exprs,
        is_empty_exprs,
        is_complete_exprs,
        extra_items,
    } = parts;

    // Prepare some values for interpolation
    let (mod_name, struct_name) = partial_names(&input.name);
    let visibility = &input.visibility;
    let partial_attrs = &input.partial_attrs;
    let struct_visibility = inner_visibility(&input.visibility, Span::call_site());
    let module_doc = format!(
        "*Generated* by `confique`: helpers to implement `Config` for [`{}`].\n\
            \n\
            Do not use directly! Only use via the `Config` and `Partial` traits \
            and what's explained in the confique documentation.
            Any other parts of this module cannot be relied on and are not part \
            of the semver guarantee of `confique`.",
        input.name,
    );

    quote! {
        #[doc = #module_doc]
        #visibility mod #mod_name {
            #![allow(missing_docs)]
            use super::*;

            #[derive(confique::serde::Deserialize)]
            #[serde(crate = "confique::serde")]
            #( #[ #partial_attrs ])*
            #struct_visibility struct #struct_name {
                #( #struct_fields )*
            }

            #[automatically_derived]
            impl confique::Partial for #struct_name where #( #nested_bounds, )* {
                fn empty() -> Self {
                    Self {
                        #( #field_names: #empty_exprs, )*
                    }
                }

                fn default_values() -> Self {
                    Self {
                        #( #field_names: #default_exprs, )*
                    }
                }

                fn from_env() -> std::result::Result<Self, confique::Error> {
                    std::result::Result::Ok(Self {
                        #( #field_names: #from_env_exprs, )*
                    })
                }

                fn with_fallback(self, fallback: Self) -> Self {
                    Self {
                        #( #field_names: #fallback_exprs, )*
                    }
                }

                fn is_empty(&self) -> bool {
                    true #(&& #is_empty_exprs)*
                }

                fn is_complete(&self) -> bool {
                    true #(&& #is_complete_exprs)*
                }
            }

            #extra_items
        }
    }
}

#[derive(Default)]
struct Parts {
    field_names: Vec<Ident>,
    struct_fields: Vec<TokenStream>,
    nested_bounds: Vec<TokenStream>,
    empty_exprs: Vec<TokenStream>,
    default_exprs: Vec<TokenStream>,
    from_env_exprs: Vec<TokenStream>,
    fallback_exprs: Vec<TokenStream>,
    is_empty_exprs: Vec<TokenStream>,
    is_complete_exprs: Vec<TokenStream>,
    extra_items: TokenStream,
}

fn gen_parts_for_field(f: &ir::Field, input: &ir::Input, parts: &mut Parts) {
    let struct_name = &input.name;
    let field_name = &f.name;
    parts.field_names.push(field_name.clone());
    let qualified_name = format!("{struct_name}::{field_name}");

    // We have to use the span of the field's name here so that error
    // messages from the `derive(serde::Deserialize)` have the correct span.
    let field_visibility = inner_visibility(&input.visibility, field_name.span());


    match &f.kind {
        // ----- Nested -------------------------------------------------------------
        FieldKind::Nested { ty } => {
            let ty_span = ty.span();
            let field_ty = quote_spanned! {ty_span=> <#ty as confique::Config>::Partial };
            parts.struct_fields.push(quote! {
                #[serde(default = "confique::Partial::empty")]
                #field_visibility #field_name: #field_ty,
            });

            parts.nested_bounds.push(quote! { #ty: confique::Config });
            parts.empty_exprs.push(quote! { confique::Partial::empty() });
            parts.default_exprs.push(quote! { confique::Partial::default_values() });
            parts.from_env_exprs.push(quote! { confique::Partial::from_env()? });
            parts.fallback_exprs.push(quote! {
                self.#field_name.with_fallback(fallback.#field_name)
            });
            parts.is_empty_exprs.push(quote! { self.#field_name.is_empty() });
            parts.is_complete_exprs.push(quote! { self.#field_name.is_complete() });
        },


        // ----- Leaf ---------------------------------------------------------------
        FieldKind::Leaf { kind, deserialize_with, validate, env, parse_env } => {
            let inner_ty = kind.inner_ty();

            // This has an ugly name to avoid clashing with imported names.
            let deserialize_fn_name = quote::format_ident!("__confique_deserialize_{field_name}");

            // We sometimes have to create extra helper functions, either to pass to
            // `serde(deserialize_with)` or to save on code duplication. We cannot just
            // pass the deserialize function given to confique to serde, as confique
            // accepts a function deserializing the inner type `T`, where serde expects
            // `Option<T>`. Also, validation should be part of the serialization, so we
            // need to create a custom function in that case too.
            if deserialize_with.is_some() || validate.is_some() {
                let deser_fn = deserialize_with.as_ref()
                    .map(|f| quote!( #f ))
                    .unwrap_or_else(|| quote! {
                        <#inner_ty as confique::serde::Deserialize>::deserialize
                    });

                let validate_code = validate.as_ref().map(|v| match v {
                    ir::FieldValidator::Fn(path) => quote_spanned! {path.span() =>
                        confique::internal::do_validate_field(&out, &#path)
                            .map_err(<D::Error as confique::serde::de::Error>::custom)?;
                    },
                    ir::FieldValidator::Simple(expr, msg) => quote! {
                        fn is_valid(#field_name: &#inner_ty) -> bool {
                            #expr
                        }
                        if !is_valid(&out) {
                            return Err(
                                <D::Error as confique::serde::de::Error>::custom(
                                    confique::internal::field_validation_err(#msg),
                                )
                            )
                        }
                    },
                });

                parts.extra_items.extend(quote! {
                    fn #deserialize_fn_name<'de, D>(
                        deserializer: D,
                    ) -> std::result::Result<std::option::Option<#inner_ty>, D::Error>
                    where
                        D: confique::serde::Deserializer<'de>,
                    {
                        let out = #deser_fn(deserializer)?;
                        #validate_code
                        std::result::Result::Ok(std::option::Option::Some(out))
                    }
                });
            }

            // Struct field definition
            parts.struct_fields.push({
                let deserialize_fn_name = deserialize_fn_name.to_string();
                let attr = if deserialize_with.is_some() || validate.is_some() {
                    quote! {
                        #[serde(default, deserialize_with = #deserialize_fn_name)]
                    }
                } else {
                    quote! {}
                };

                let main = quote_spanned! {field_name.span()=>
                    #field_visibility #field_name: std::option::Option<#inner_ty>,
                };
                quote! { #attr #main }
            });


            // Some simple ones
            parts.empty_exprs.push(quote! { std::option::Option::None });
            parts.fallback_exprs.push(quote! { self.#field_name.or(fallback.#field_name) });
            parts.is_empty_exprs.push(quote! { self.#field_name.is_none() });
            if kind.is_required() {
                parts.is_complete_exprs.push(quote! { self.#field_name.is_some() });
            }

            // Code for `Partial::default_values()`
            parts.default_exprs.push(match kind {
                LeafKind::Required { default: Some(default), .. } => {
                    let msg = format!("default config value for `{qualified_name}` \
                        cannot be deserialized");
                    let expr = default_value_to_deserializable_expr(&default);

                    let inner = match deserialize_with {
                        None => quote! { confique::internal::deserialize_default(#expr) },
                        Some(p) => quote! { #p(confique::internal::into_deserializer(#expr)) },
                    };

                    quote! {
                        std::option::Option::Some(#inner.expect(#msg))
                    }
                }
                _ => quote! { std::option::Option::None },
            });

            // Code for `Partial::from_env()`
            parts.from_env_exprs.push(match env {
                None => quote! { std::option::Option::None },
                Some(key) => {
                    match (parse_env, deserialize_with) {
                        (None, None) => quote! {
                            confique::internal::from_env(#key, #qualified_name)?
                        },
                        (None, Some(deserialize_with)) => quote! {
                            confique::internal::from_env_with_deserializer(
                                #key, #qualified_name, #deserialize_with)?
                        },
                        (Some(parse_env), _) => quote! {
                            confique::internal::from_env_with_parser(
                                #key, #qualified_name, #parse_env)?
                        },
                    }
                },
            });
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
