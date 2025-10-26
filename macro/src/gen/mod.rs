use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, spanned::Spanned};

use crate::ir::{self, FieldKind, LeafKind};

mod meta;


/// The main function to generate the output token stream from the parse IR.
pub(crate) fn gen(input: ir::Input) -> TokenStream {
    let layer_mod = gen_layer_mod(&input);
    let config_impl = gen_config_impl(&input);

    quote! {
        #config_impl
        #layer_mod
    }
}

/// Generates the `impl Config for ... { ... }`.
fn gen_config_impl(input: &ir::Input) -> TokenStream {
    let name = &input.name;
    let (layer_mod_name, layer_struct_name) = layer_names(&input.name);

    let field_names = input.fields.iter().map(|f| &f.name);
    let from_exprs = input.fields.iter().map(|f| {
        let field_name = &f.name;
        let path = field_name.to_string();
        match f.kind {
            FieldKind::Nested { .. } => {
                quote! {
                    confique::internal::map_err_prefix_path(
                        confique::Config::from_layer(layer.#field_name),
                        #path,
                    )?
                }
            }
            FieldKind::Leaf { kind: LeafKind::Optional { .. }, .. } => {
                quote! { layer.#field_name }
            }
            FieldKind::Leaf { kind: LeafKind::Required { .. }, .. } => {
                quote! {
                    confique::internal::unwrap_or_missing_value_err(layer.#field_name, #path)?
                }
            }
        }
    });

    let validation = input.validate.as_ref().map(|v| {
        let struct_name = name.to_string();
        quote! {
            confique::internal::validate_struct(&out, &#v, #struct_name)?;
        }
    });

    let meta_item = meta::gen(input);
    quote! {
        #[automatically_derived]
        impl confique::Config for #name {
            type Layer = #layer_mod_name::#layer_struct_name;

            fn from_layer(layer: Self::Layer) -> std::result::Result<Self, confique::Error> {
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

/// Generates the whole `mod ... { ... }` that defines the layer type and
/// related items.
fn gen_layer_mod(input: &ir::Input) -> TokenStream {
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
    let (mod_name, struct_name) = layer_names(&input.name);
    let visibility = &input.visibility;
    let layer_attrs = &input.layer_attrs;
    let struct_visibility = inner_visibility(&input.visibility, Span::call_site());
    let module_doc = format!(
        "*Generated* by `confique`: helpers to implement `Config` for [`{}`].\n\
            \n\
            Do not use directly! Only use via the `Config` and `Layer` traits \
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
            #( #[ #layer_attrs ])*
            #struct_visibility struct #struct_name {
                #( #struct_fields )*
            }

            #[automatically_derived]
            impl confique::Layer for #struct_name where #( #nested_bounds, )* {
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
            let field_ty = quote_spanned! {ty_span=> <#ty as confique::Config>::Layer };
            let layer_attrs = &f.layer_attrs;
            let docs = &f.doc;
            parts.struct_fields.push(quote! {
                #( #[ #layer_attrs ])*
                #( #[ doc = #docs ] )*
                #[serde(default = "confique::Layer::empty")]
                #field_visibility #field_name: #field_ty,
            });

            parts.nested_bounds.push(quote! { #ty: confique::Config });
            parts.empty_exprs.push(quote! { confique::Layer::empty() });
            parts.default_exprs.push(quote! { confique::Layer::default_values() });
            parts.from_env_exprs.push(quote! { confique::Layer::from_env()? });
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
            let validate_fn_name = quote::format_ident!("__confique_validate_{field_name}");
            let deserialize_fn_name
                = quote::format_ident!("__confique_deserialize_direct_{field_name}");

            let default_deserialize_path = quote! {
                <#inner_ty as confique::serde::Deserialize>::deserialize
            };

            // We sometimes emit extra helper functions to avoid code duplication.
            // Validation should be part of the serialization. `validation_fn` is
            // `Some(Ident)` if there is a validator function. `deserialize_fn` is
            // a token stream that represents a callable function that deserializes
            // `inner_ty`.
            let (validate_fn, deserialize_fn) = if validate.len() > 0 {
                let validate_inner = validate.iter().map(|validator| match validator {
                    ir::FieldValidator::Fn(f) => quote_spanned! {f.span() =>
                        confique::internal::validate_field(v, &#f)?;
                    },
                    ir::FieldValidator::Simple(expr, msg) => quote! {
                        confique::internal::validate_field(v, &|v| {
                            fn is_valid(#field_name: &#inner_ty) -> bool {
                                #expr
                            }
                            if !is_valid(v) {
                                return Err(#msg);
                            } else {
                                Ok(())
                            }
                        })?;
                    },
                }).collect::<TokenStream>();

                let deser_fn = deserialize_with.as_ref()
                    .map(|f| quote!( #f ))
                    .unwrap_or_else(|| default_deserialize_path.clone());

                parts.extra_items.extend(quote! {
                    #[inline(never)]
                    fn #validate_fn_name(
                        v: &#inner_ty,
                    ) -> std::result::Result<(), confique::Error> {
                        #validate_inner
                        Ok(())
                    }

                    fn #deserialize_fn_name<'de, D>(
                        deserializer: D,
                    ) -> std::result::Result<#inner_ty, D::Error>
                    where
                        D: confique::serde::Deserializer<'de>,
                    {
                        let out = #deser_fn(deserializer)?;
                        #validate_fn_name(&out)
                            .map_err(<D::Error as confique::serde::de::Error>::custom)?;
                        std::result::Result::Ok(out)
                    }
                });

                (Some(validate_fn_name), quote! { #deserialize_fn_name })
            } else {
                // If there is no validation, we will not create a custom
                // deserialization function for this, so we either use `T::deserialize`
                // or, if set, the specified deserialization function.
                let deser = deserialize_with.as_ref()
                    .map(|f| quote! { #f })
                    .unwrap_or(default_deserialize_path);
                (None, deser)
            };


            // Struct field definition
            parts.struct_fields.push({
                // If there is a custom deserializer or a validator, we need to
                // set the serde `deserialize_with` attribute.
                let attr = if deserialize_with.is_some() || validate.len() > 0 {
                    // Since the struct field is `Option<T>`, we need to create
                    // another wrapper deserialization function, that always
                    // returns `Some`.
                    let fn_name = quote::format_ident!("__confique_deserialize_some_{field_name}");
                    parts.extra_items.extend(quote! {
                        fn #fn_name<'de, D>(
                            deserializer: D,
                        ) -> std::result::Result<Option<#inner_ty>, D::Error>
                        where
                            D: confique::serde::Deserializer<'de>,
                        {
                            #deserialize_fn(deserializer).map(Option::Some)
                        }
                    });

                    let attr_value = fn_name.to_string();
                    quote! {
                        #[serde(default, deserialize_with = #attr_value)]
                    }
                } else {
                    quote! {}
                };

                let main = quote_spanned! {field_name.span()=>
                    #field_visibility #field_name: Option<#inner_ty>,
                };
                let layer_attrs = &f.layer_attrs;
                let docs = &f.doc;
                quote! {
                    #attr
                    #( #[ #layer_attrs ])*
                    #( #[ doc = #docs ] )*
                    #main
                }
            });


            // Some simple ones
            parts.empty_exprs.push(quote! { Option::None });
            parts.fallback_exprs.push(quote! { self.#field_name.or(fallback.#field_name) });
            parts.is_empty_exprs.push(quote! { self.#field_name.is_none() });
            if kind.is_required() {
                parts.is_complete_exprs.push(quote! { self.#field_name.is_some() });
            }

            // Code for `Layer::default_values()`
            parts.default_exprs.push(match kind {
                LeafKind::Required { default: Some(default), .. } => {
                    let msg = format!("default config value for `{qualified_name}` \
                        cannot be deserialized");
                    let expr = default_value_to_deserializable_expr(&default);
                    quote! {
                        Option::Some(
                            #deserialize_fn(confique::internal::into_deserializer(#expr))
                                .expect(#msg)
                        )
                    }
                }
                _ => quote! { Option::None },
            });

            // Code for `Layer::from_env()`
            parts.from_env_exprs.push(match (env, parse_env) {
                (None, _) => quote! { Option::None },
                (Some(key), None) => quote! {
                    confique::internal::from_env(#key, #qualified_name, #deserialize_fn)?
                },
                (Some(key), Some(parse_env)) => {
                    let validator = match &validate_fn {
                        Some(f) => quote! { #f },
                        None => quote! { |_| std::result::Result::<(), String>::Ok(()) },
                    };
                    quote! {
                        confique::internal::from_env_with_parser(
                            #key, #qualified_name, #parse_env, #validator)?
                    }
                }
            });
        }
    }
}

/// Returns the names of the module and struct for the layer type:
/// `(mod_name, struct_name)`.
fn layer_names(original_name: &Ident) -> (Ident, Ident) {
    use heck::ToSnakeCase;
    (
        format_ident!("confique_{}_layer", original_name.to_string().to_snake_case()),
        format_ident!("{original_name}Layer"),
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
        syn::Visibility::Public(_) => quote_spanned! {span=> #outer },
        syn::Visibility::Restricted(r) if r.path.is_ident("crate") && r.in_token.is_none() => {
            quote_spanned! {span=> #outer }
        },

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
