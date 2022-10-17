//! Definition of the intermediate representation.

use syn::{Error, Token, parse::{Parse, ParseStream}, spanned::Spanned, punctuated::Punctuated};

use crate::util::{is_option, unwrap_option};


/// The parsed input to the `gen_config` macro.
pub(crate) struct Input {
    pub(crate) doc: Vec<String>,
    pub(crate) visibility: syn::Visibility,
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
        kind: LeafKind,
    },

    /// A nested configuration. The type is never `Option<_>`.
    Nested {
        ty: syn::Type,
    },
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
}

impl Input {
    pub(crate) fn from_ast(mut input: syn::DeriveInput) -> Result<Self, Error> {
        let fields = match input.data {
            syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(f), .. }) => f,
            _ => return Err(Error::new(
                input.span(),
                "`confique::Config` can only be derive for structs with named fields",
            )),
        };

        let doc = extract_doc(&mut input.attrs);
        let fields = fields.named.into_iter()
            .map(Field::from_ast)
            .collect::<Result<Vec<_>, _>>()?;


        Ok(Self {
            doc,
            visibility: input.vis,
            name: input.ident,
            fields,
        })
    }
}

impl Field {
    fn from_ast(mut field: syn::Field) -> Result<Self, Error> {
        let doc = extract_doc(&mut field.attrs);
        let attrs = extract_internal_attrs(&mut field.attrs)?;

        // TODO: check no other attributes are here
        let kind = if attrs.nested {
            if is_option(&field.ty) {
                return Err(Error::new(
                    field.ident.span(),
                    "nested configurations cannot be optional (type `Option<_>`)",
                ));
            }
            if attrs.default.is_some() {
                return Err(Error::new(
                    field.ident.span(),
                    "cannot specify `nested` and `default` attributes at the same time",
                ));
            }
            if attrs.env.is_some() {
                return Err(Error::new(
                    field.ident.span(),
                    "cannot specify `nested` and `env` attributes at the same time",
                ));
            }
            if attrs.deserialize_with.is_some() {
                return Err(Error::new(
                    field.ident.span(),
                    "cannot specify `nested` and `deserialize_with` attributes at the same time",
                ));
            }

            FieldKind::Nested { ty: field.ty }
        } else {
            match unwrap_option(&field.ty) {
                None => FieldKind::Leaf {
                    env: attrs.env,
                    deserialize_with: attrs.deserialize_with,
                    kind: LeafKind::Required {
                        default: attrs.default,
                        ty: field.ty,
                    },
                },
                Some(inner) => {
                    if attrs.default.is_some() {
                        return Err(Error::new(
                            field.ident.span(),
                            "optional fields (type `Option<_>`) cannot have default \
                                values (`#[config(default = ...)]`)",
                        ));
                    }

                    FieldKind::Leaf {
                        env: attrs.env,
                        deserialize_with: attrs.deserialize_with,
                        kind: LeafKind::Optional {
                            inner_ty: inner.clone(),
                        },
                    }
                }
            }
        };

        Ok(Self {
            doc,
            name: field.ident.expect("bug: expected named field"),
            kind,
        })
    }

    pub(crate) fn is_leaf(&self) -> bool {
        matches!(self.kind, FieldKind::Leaf { .. })
    }
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let msg = "invalid default value. Allowed are only: certain literals \
            (string, integer, float, bool), and arrays";

        if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);

            let items = <Punctuated<Expr, Token![,]>>::parse_terminated(&content)?;
            Ok(Self::Array(items.into_iter().collect()))
        } else {
            let lit: syn::Lit = input.parse()
                .map_err(|_| Error::new(input.span(), msg))?;
            match lit {
                syn::Lit::Str(l) => Ok(Self::Str(l)),
                syn::Lit::Int(l) => Ok(Self::Int(l)),
                syn::Lit::Float(l) => Ok(Self::Float(l)),
                syn::Lit::Bool(l) => Ok(Self::Bool(l)),
                _ => Err(Error::new(lit.span(), msg)),
            }
        }
    }
}

/// Extracts all doc string attributes from the list and return them as list of
/// strings (in order).
fn extract_doc(attrs: &mut Vec<syn::Attribute>) -> Vec<String> {
    extract_attrs(attrs, |attr| {
        match attr.parse_meta().ok()? {
            syn::Meta::NameValue(syn::MetaNameValue {
                lit: syn::Lit::Str(s),
                path,
                ..
            }) if path.is_ident("doc") => Some(s.value()),
            _ => None,
        }
    })
}

fn extract_attrs<P, O>(attrs: &mut Vec<syn::Attribute>, mut pred: P) -> Vec<O>
where
    P: FnMut(&syn::Attribute) -> Option<O>,
{
    // TODO: use `Vec::drain_filter` once stabilized. The current impl is O(n²).
    let mut i = 0;
    let mut out = Vec::new();
    while i < attrs.len() {
        match pred(&attrs[i]) {
            Some(v) => {
                out.push(v);
                attrs.remove(i);
            }
            None => i += 1,
        }
    }

    out
}

fn extract_internal_attrs(
    attrs: &mut Vec<syn::Attribute>,
) -> Result<InternalAttrs, Error> {
    let internal_attrs = extract_attrs(attrs, |attr| {
        if attr.path.is_ident("config") {
            // TODO: clone not necessary once we use drain_filter
            Some(attr.clone())
        } else {
            None
        }
    });


    let mut out = InternalAttrs::default();
    for attr in internal_attrs {
        type AttrList = Punctuated<InternalAttr, Token![,]>;
        let parsed_list = attr.parse_args_with(AttrList::parse_terminated)?;

        for parsed in parsed_list {
            let keyword = parsed.keyword();

            macro_rules! duplicate_if {
                ($cond:expr) => {
                    if $cond {
                        let msg = format!("duplicate '{keyword}' confique attribute");
                        return Err(Error::new(attr.tokens.span(), msg));
                    }
                };
            }

            match parsed {
                InternalAttr::Default(expr) => {
                    duplicate_if!(out.default.is_some());
                    out.default = Some(expr);
                }
                InternalAttr::Nested => {
                    duplicate_if!(out.nested);
                    out.nested = true;
                }
                InternalAttr::Env(key) => {
                    duplicate_if!(out.env.is_some());
                    out.env = Some(key);
                }
                InternalAttr::DeserializeWith(path) => {
                    duplicate_if!(out.deserialize_with.is_some());
                    out.deserialize_with = Some(path);
                }
            }
        }
    }

    Ok(out)
}

#[derive(Default)]
struct InternalAttrs {
    nested: bool,
    default: Option<Expr>,
    env: Option<String>,
    deserialize_with: Option<syn::Path>,
}

enum InternalAttr {
    Nested,
    Default(Expr),
    Env(String),
    DeserializeWith(syn::Path),
}

impl InternalAttr {
    fn keyword(&self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Default(_) => "default",
            Self::Env(_) => "env",
            Self::DeserializeWith(_) => "deserialize_with",
        }
    }
}

impl Parse for InternalAttr {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident: syn::Ident = input.parse()?;
        match &*ident.to_string() {
            "nested" => {
                assert_empty_or_comma(input)?;
                Ok(Self::Nested)
            }

            "default" => {
                let _: Token![=] = input.parse()?;
                let expr: Expr = input.parse()?;
                assert_empty_or_comma(input)?;
                Ok(Self::Default(expr))
            }

            "env" => {
                let _: Token![=] = input.parse()?;
                let key: syn::LitStr = input.parse()?;
                assert_empty_or_comma(input)?;
                let value = key.value();
                if value.contains('=') || value.contains('\0') {
                    Err(syn::Error::new(
                        key.span(),
                        "environment variable key must not contain '=' or null bytes",
                    ))
                } else {
                    Ok(Self::Env(value))
                }
            }

            "deserialize_with" => {
                let _: Token![=] = input.parse()?;
                let path: syn::Path = input.parse()?;
                assert_empty_or_comma(input)?;

                Ok(Self::DeserializeWith(path))
            }

            _ => Err(syn::Error::new(ident.span(), "unknown confique attribute")),
        }
    }
}

fn assert_empty_or_comma(input: ParseStream) -> Result<(), Error> {
    if input.is_empty() || input.peek(Token![,]) {
        Ok(())
    } else {
        Err(Error::new(input.span(), "unexpected tokens, expected no more tokens in this context"))
    }
}
