use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, ext::IdentExt, spanned::Spanned};

use crate::ir::{self, FieldKind, LeafKind};

mod meta;


/// The main function to generate the output token stream from the parse IR.
pub(crate) fn gen(input: ir::Input) -> TokenStream {
    let layer = gen_layer(&input);
    let config_impl = gen_config_impl(&input);

    quote! {
        #config_impl
        #layer
    }
}

/// Generates the `impl Config for ... { ... }`.
fn gen_config_impl(input: &ir::Input) -> TokenStream {
    let name = &input.name;
    let layer_name = layer_name(input);

    let field_names = input.fields.iter().map(|f| &f.name);
    let from_exprs = input.fields.iter().map(|f| {
        let field_name = &f.name;
        let path = field_name.unraw().to_string();
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
        let struct_name = name.unraw().to_string();
        quote! {
            confique::internal::validate_struct(&out, &#v, #struct_name)?;
        }
    });

    let meta_item = meta::gen(input);
    quote! {
        #[automatically_derived]
        impl confique::Config for #name {
            type Layer = #layer_name;

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

/// Generates the layer type and its impls.
fn gen_layer(input: &ir::Input) -> TokenStream {
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
        validate_exprs,
        deserialize_into_methods,
        layer_methods,
    } = parts;

    // Prepare some values for interpolation
    let struct_name = layer_name(input);
    let visibility = &input.visibility;
    let layer_attrs = &input.layer_attrs;
    let doc = format!(
        "Layer type of [`{}`], *generated* by `confique`. \
            See [`confique::Layer`] for more information.",
        input.name.unraw(),
    );

    quote! {
        #[doc = #doc]
        #[allow(missing_docs)]
        #[derive(confique::serde::Deserialize)]
        #[serde(crate = "confique::serde")]
        #( #[ #layer_attrs ])*
        #visibility struct #struct_name {
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

            fn validate(&self) -> std::result::Result<(), confique::Error> {
                #( #validate_exprs )*
                std::result::Result::Ok(())
            }
        }

        #[automatically_derived]
        #[allow(non_snake_case)]
        impl #struct_name {
            #( #deserialize_into_methods )*
            #layer_methods
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
    validate_exprs: Vec<TokenStream>,

    /// The `__confique_deserialize_into_{field}` methods of the layer, one per field.
    /// Currently only used by `layer!`.
    deserialize_into_methods: Vec<TokenStream>,

    /// Other inherent methods of the layer type (e.g. `validate_{field}` and
    /// private helpers).
    layer_methods: TokenStream,
}

fn gen_parts_for_field(f: &ir::Field, input: &ir::Input, parts: &mut Parts) {
    let struct_name = &input.name;
    let layer_struct_name = layer_name(input);
    let field_name = &f.name;
    parts.field_names.push(field_name.clone());

    // Names without `r#` prefix, for building strings and new identifiers.
    let unraw_field_name = field_name.unraw();
    let path = unraw_field_name.to_string();
    let qualified_name = format!("{}::{unraw_field_name}", struct_name.unraw());

    // We have to use the span of the field's name here so that error
    // messages from the `derive(serde::Deserialize)` have the correct span.
    let visibility = &input.visibility;
    let field_visibility = quote_spanned! {field_name.span()=> #visibility };


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
            parts.validate_exprs.push(quote! {
                confique::internal::map_err_prefix_path(
                    confique::Layer::validate(&self.#field_name),
                    #path,
                )?;
            });
            parts.deserialize_into_methods.push(gen_deserialize_into_method(
                field_name,
                &field_visibility,
                &quote! { confique::serde::Deserialize::deserialize },
                false,
            ));
        },


        // ----- Leaf ---------------------------------------------------------------
        FieldKind::Leaf { kind, deserialize_with, validate, env, parse_env } => {
            let inner_ty = kind.inner_ty();

            let validate_fn_name = quote::format_ident!("validate_{unraw_field_name}");
            let deserialize_fn_name = quote::format_ident!("deserialize_{unraw_field_name}");

            // The public `validate_{field}` method, generated for every leaf
            // field. Without validators, it's a no-op.
            let has_validators = !validate.is_empty();
            {
                let validate_inner = validate.iter().map(|validator| match validator {
                    ir::FieldValidator::Fn(f) => quote_spanned! {f.span() =>
                        confique::internal::validate_field(v, &#f, #path)?;
                    },
                    ir::FieldValidator::Simple(expr, msg) => quote! {
                        confique::internal::validate_field(v, &|v| {
                            fn is_valid(#field_name: &#inner_ty) -> bool {
                                #expr
                            }
                            if !is_valid(v) {
                                return std::result::Result::Err(#msg);
                            } else {
                                std::result::Result::Ok(())
                            }
                        }, #path)?;
                    },
                }).collect::<TokenStream>();

                let (doc, validate_inner) = if has_validators {
                    let doc = format!(
                        "Runs all validators of the field `{unraw_field_name}` (specified via \
                            `#[config(validate = ...)]`) on the given value.",
                    );
                    (doc, validate_inner)
                } else {
                    let doc = format!(
                        "No-op that always returns `Ok(())`, as the field \
                            `{unraw_field_name}` has no validators.",
                    );
                    (doc, quote! { let _ = v; })
                };
                parts.layer_methods.extend(quote! {
                    #[doc = #doc]
                    #visibility fn #validate_fn_name(
                        v: &#inner_ty,
                    ) -> std::result::Result<(), confique::Error> {
                        #validate_inner
                        std::result::Result::Ok(())
                    }
                });
            }
            let validate_fn = quote! { #layer_struct_name::#validate_fn_name };

            // The public `deserialize_{field}` method.
            let deser_fn = match deserialize_with {
                Some(f) => quote! { #f },
                None => quote! { <#inner_ty as confique::serde::Deserialize>::deserialize },
            };
            let body = match has_validators {
                false => quote! { #deser_fn(deserializer) },
                true => quote! {
                    let out = #deser_fn(deserializer)?;
                    #validate_fn(&out).map_err(<D::Error as confique::serde::de::Error>::custom)?;
                    std::result::Result::Ok(out)
                },
            };
            let doc = {
                let deser_desc = match deserialize_with {
                    // Token streams are stringified with spaces around `::` etc.
                    Some(f) => format!(
                        "`{}` (specified via `deserialize_with`)",
                        quote!(#f).to_string().replace(' ', ""),
                    ),
                    None => "`serde::Deserialize::deserialize`".into(),
                };
                let action = match has_validators {
                    true => format!(
                        "calls {deser_desc} and then validates the value via \
                            [`Self::{validate_fn_name}`]",
                    ),
                    false => format!("simply calls {deser_desc}"),
                };

                format!(
                    "Deserializes a value for the field `{unraw_field_name}` like confique does \
                        when loading a layer: {action}.",
                )
            };
            parts.layer_methods.extend(quote! {
                #[doc = #doc]
                #visibility fn #deserialize_fn_name<'de, D>(
                    deserializer: D,
                ) -> std::result::Result<#inner_ty, D::Error>
                where
                    D: confique::serde::Deserializer<'de>,
                {
                    #body
                }
            });
            let deserialize_fn = quote! { #layer_struct_name::#deserialize_fn_name };


            // Struct field definition
            parts.struct_fields.push({
                // If there is a custom deserializer or a validator, we need to
                // set the serde `deserialize_with` attribute.
                let attr = if deserialize_with.is_some() || has_validators {
                    // Since the struct field is `Option<T>`, we need to create
                    // another wrapper deserialization function. It goes through
                    // `Option`'s `Deserialize` impl to get serde's standard
                    // handling: `null` results in `None` and only other values
                    // are passed to the field's deserialization function.
                    let fn_name
                        = quote::format_ident!("__confique_deserialize_some_{unraw_field_name}");
                    parts.layer_methods.extend(quote! {
                        #[doc(hidden)]
                        fn #fn_name<'de, D>(
                            deserializer: D,
                        ) -> std::result::Result<Option<#inner_ty>, D::Error>
                        where
                            D: confique::serde::Deserializer<'de>,
                        {
                            struct __ConfiqueWrap(#inner_ty);
                            impl<'a> confique::serde::Deserialize<'a> for __ConfiqueWrap {
                                fn deserialize<D2>(
                                    deserializer: D2,
                                ) -> std::result::Result<Self, D2::Error>
                                where
                                    D2: confique::serde::Deserializer<'a>,
                                {
                                    #deserialize_fn(deserializer).map(__ConfiqueWrap)
                                }
                            }

                            <Option<__ConfiqueWrap>
                                as confique::serde::Deserialize>::deserialize(deserializer)
                                .map(|o| o.map(|w| w.0))
                        }
                    });

                    let attr_value = format!("{layer_struct_name}::{fn_name}");
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

            // Code for `Layer::validate()`
            if has_validators {
                parts.validate_exprs.push(quote! {
                    if let Option::Some(v) = &self.#field_name {
                        #validate_fn(v)?;
                    }
                });
            }

            parts.deserialize_into_methods.push(
                gen_deserialize_into_method(field_name, &field_visibility, &deserialize_fn, true)
            );

            // Code for `Layer::default_values()`
            parts.default_exprs.push(match kind {
                LeafKind::Required { default: Some(default), .. } => {
                    let msg = format!("default config value for `{qualified_name}` \
                        cannot be deserialized");
                    let expr = deserializable_expr(&default);
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
                (Some(key), Some(parse_env)) => quote! {
                    confique::internal::from_env_with_parser(
                        #key, #qualified_name, #parse_env, #validate_fn)?
                },
            });
        }
    }
}

/// Returns the name of the layer type: specified via `layer_name` or
/// `{name}Layer` by default.
fn layer_name(input: &ir::Input) -> Ident {
    input.layer_name.clone().unwrap_or_else(|| format_ident!("{}Layer", input.name.unraw()))
}

/// Returns the name of the layer's method that deserializes into the given
/// field.
pub(crate) fn deserialize_into_method_name(field_name: &Ident) -> Ident {
    format_ident!("__confique_deserialize_into_{}", field_name.unraw(), span = field_name.span())
}

/// Generates the `__confique_deserialize_into_{field}` method of the layer for one field:
/// it deserializes a value via `deserialize_fn` and stores it in that field.
fn gen_deserialize_into_method(
    field_name: &Ident,
    visibility: &TokenStream,
    deserialize_fn: &TokenStream,
    is_leaf: bool,
) -> TokenStream {
    let method_name = deserialize_into_method_name(field_name);
    let mut value = quote! { #deserialize_fn(deserializer)? };
    if is_leaf {
        value = quote! { Option::Some(#value) };
    }

    quote! {
        #[doc(hidden)]
        #visibility fn #method_name<'de, D>(
            &mut self,
            deserializer: D,
        ) -> std::result::Result<(), D::Error>
        where
            D: confique::serde::Deserializer<'de>,
        {
            self.#field_name = #value;
            std::result::Result::Ok(())
        }
    }
}

/// Generates a Rust expression from the value (a default value or a value in
/// `layer!`) that implements `serde::de::IntoDeserializer`.
pub(crate) fn deserializable_expr(expr: &ir::Expr) -> TokenStream {
    match expr {
        ir::Expr::Rust(tokens) => tokens.clone(),
        ir::Expr::Str(lit) => quote! { #lit },
        ir::Expr::Int(lit, None) => quote! { #lit },
        ir::Expr::Int(lit, Some(ty)) => {
            // Without suffix, the literal would be `i32`, which fails to
            // compile for values outside that range.
            let repr = format!("{}{}", lit.base10_digits(), ty.suffix());
            let lit = syn::LitInt::new(&repr, lit.span());
            quote! { #lit }
        },
        ir::Expr::Float(lit, _) => quote! { #lit },
        ir::Expr::Bool(lit) => quote! { #lit },

        // Arrays and maps are dynamically typed, as their items can have
        // different types.
        ir::Expr::Array(arr) => {
            let items = arr.iter().map(value_expr);
            quote! { confique::internal::Value::Seq(std::vec![ #(#items),* ]) }
        },
        ir::Expr::Map(entries) => {
            let items = entries.iter().map(|e| {
                let key = value_expr(&e.key.clone().into());
                let value = value_expr(&e.value);
                quote! { (#key, #value) }
            });
            quote! { confique::internal::Value::Map(std::vec![ #(#items),* ]) }
        },
    }
}

/// Like `deserializable_expr`, but the expression is always of type
/// `confique::internal::Value`.
fn value_expr(expr: &ir::Expr) -> TokenStream {
    let out = deserializable_expr(expr);
    match expr {
        ir::Expr::Array(_) | ir::Expr::Map(_) => out,
        _ => quote! { confique::internal::Value::new(#out) },
    }
}
