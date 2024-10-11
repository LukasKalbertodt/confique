use proc_macro2::{TokenStream, Group, Delimiter, Ident};
use syn::{Error, Token, parse::{Parse, ParseStream}, spanned::Spanned, punctuated::Punctuated};

use crate::{
    ir::{Input, Field, FieldKind, LeafKind, Expr, MapEntry, MapKey},
    util::{unwrap_option, is_option},
};


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
        let attrs = StructAttrs::extract(&mut input.attrs)?;
        let fields = fields.named.into_iter()
            .map(Field::from_ast)
            .collect::<Result<Vec<_>, _>>()?;


        Ok(Self {
            doc,
            visibility: input.vis,
            partial_attrs: attrs.partial_attrs,
            name: input.ident,
            fields,
        })
    }
}

// ===== Attributes on the struct =====================================================

#[derive(Default)]
struct StructAttrs {
    partial_attrs: Vec<TokenStream>,
}

#[derive(Debug)]
enum StructAttr {
    PartialAttrs(TokenStream),
}

impl StructAttrs {
    fn extract(attrs: &mut Vec<syn::Attribute>) -> Result<Self, Error> {
        let attrs = extract_config_attrs(attrs);

        let mut out = Self::default();
        for attr in attrs {
            type AttrList = Punctuated<StructAttr, Token![,]>;
            let parsed_list = attr.parse_args_with(AttrList::parse_terminated)?;

            for parsed in parsed_list {
                match parsed {
                    StructAttr::PartialAttrs(tokens) => out.partial_attrs.push(tokens),
                }
            }
        }

        Ok(out)
    }
}

impl Parse for StructAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match &*ident.to_string() {
            "partial_attr" => {
                let g: Group = input.parse()?;
                if g.delimiter() != Delimiter::Parenthesis {
                    return Err(Error::new_spanned(g,
                        "expected `(...)` but found different delimiter"));
                }
                assert_empty_or_comma(&input)?;
                Ok(Self::PartialAttrs(g.stream()))
            }
            _ => Err(syn::Error::new(ident.span(), "unknown confique attribute")),
        }
    }
}


// ===== Struct fields =============================================================

impl Field {
    fn from_ast(mut field: syn::Field) -> Result<Self, Error> {
        let doc = extract_doc(&mut field.attrs);
        let attrs = FieldAttrs::extract(&mut field.attrs)?;

        let err = |msg| Err(Error::new(field.ident.span(), msg));

        // TODO: check no other attributes are here
        let kind = if attrs.nested {
            if is_option(&field.ty) {
                return err("nested configurations cannot be optional (type `Option<_>`)");
            }
            if attrs.default.is_some() {
                return err("cannot specify `nested` and `default` attributes at the same time");
            }
            if attrs.env.is_some() {
                return err("cannot specify `nested` and `env` attributes at the same time");
            }
            if attrs.deserialize_with.is_some() {
                return err("cannot specify `nested` and `deserialize_with` attributes \
                    at the same time");
            }

            FieldKind::Nested { ty: field.ty }
        } else {
            if attrs.env.is_none() && attrs.parse_env.is_some() {
                return err("cannot specify `parse_env` attribute without the `env` attribute");
            }

            let kind = match unwrap_option(&field.ty) {
                Some(_) if attrs.default.is_some() => {
                    return err("optional fields (type `Option<_>`) cannot have default \
                            values (`#[config(default = ...)]`)");
                },
                Some(inner) => LeafKind::Optional { inner_ty: inner.clone() },
                None => LeafKind::Required { default: attrs.default, ty: field.ty },
            };

            FieldKind::Leaf {
                env: attrs.env,
                deserialize_with: attrs.deserialize_with,
                parse_env: attrs.parse_env,
                kind,
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


// ===== Attributes on fields =====================================================

#[derive(Default)]
struct FieldAttrs {
    nested: bool,
    default: Option<Expr>,
    env: Option<String>,
    deserialize_with: Option<syn::Path>,
    parse_env: Option<syn::Path>,
}

enum FieldAttr {
    Nested,
    Default(Expr),
    Env(String),
    DeserializeWith(syn::Path),
    ParseEnv(syn::Path),
}

impl FieldAttrs {
    fn extract(attrs: &mut Vec<syn::Attribute>) -> Result<Self, Error> {
        let attrs = extract_config_attrs(attrs);

        let mut out = FieldAttrs::default();
        for attr in attrs {
            type AttrList = Punctuated<FieldAttr, Token![,]>;
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
                    FieldAttr::Default(expr) => {
                        duplicate_if!(out.default.is_some());
                        out.default = Some(expr);
                    }
                    FieldAttr::Nested => {
                        duplicate_if!(out.nested);
                        out.nested = true;
                    }
                    FieldAttr::Env(key) => {
                        duplicate_if!(out.env.is_some());
                        out.env = Some(key);
                    }
                    FieldAttr::ParseEnv(path) => {
                        duplicate_if!(out.parse_env.is_some());
                        out.parse_env = Some(path);
                    }
                    FieldAttr::DeserializeWith(path) => {
                        duplicate_if!(out.deserialize_with.is_some());
                        out.deserialize_with = Some(path);
                    }
                }
            }
        }

        Ok(out)
    }
}

impl FieldAttr {
    fn keyword(&self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Default(_) => "default",
            Self::Env(_) => "env",
            Self::ParseEnv(_) => "parse_env",
            Self::DeserializeWith(_) => "deserialize_with",
        }
    }
}

impl Parse for FieldAttr {
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

            "parse_env" => {
                let _: Token![=] = input.parse()?;
                let path: syn::Path = input.parse()?;
                assert_empty_or_comma(input)?;

                Ok(Self::ParseEnv(path))
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


// ===== Expr =====================================================================

impl Parse for Expr {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let msg = "invalid default value. Allowed are only: certain literals \
            (string, integer, float, bool), and arrays";

        if input.peek(syn::token::Bracket) {
            // ----- Array -----
            let content;
            syn::bracketed!(content in input);

            let items = <Punctuated<Expr, Token![,]>>::parse_terminated(&content)?;
            Ok(Self::Array(items.into_iter().collect()))
        } else if input.peek(syn::token::Brace) {
            // ----- Map -----
            let content;
            syn::braced!(content in input);

            let items = <Punctuated<MapEntry, Token![,]>>::parse_terminated(&content)?;
            Ok(Self::Map(items.into_iter().collect()))
        } else {
            // ----- Literal -----

            // We just use `MapKey` here as it's exactly what we want, despite
            // this not having anything to do with maps.
            input.parse::<MapKey>()
                .map_err(|_| Error::new(input.span(), msg))
                .map(Into::into)
        }
    }
}

impl Parse for MapEntry {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let key: MapKey = input.parse()?;
        let _: syn::Token![:] = input.parse()?;
        let value: Expr = input.parse()?;
        Ok(Self { key, value })
    }
}

impl Parse for MapKey {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lit: syn::Lit = input.parse()?;
        match lit {
            syn::Lit::Str(l) => Ok(Self::Str(l)),
            syn::Lit::Int(l) => Ok(Self::Int(l)),
            syn::Lit::Float(l) => Ok(Self::Float(l)),
            syn::Lit::Bool(l) => Ok(Self::Bool(l)),
            _ => Err(Error::new(
                lit.span(),
                "only string, integer, float, and Boolean literals allowed as map key",
            )),
        }
    }
}


// ===== Util =====================================================================

fn assert_empty_or_comma(input: ParseStream) -> Result<(), Error> {
    if input.is_empty() || input.peek(Token![,]) {
        Ok(())
    } else {
        Err(Error::new(input.span(), "unexpected tokens, expected no more tokens in this context"))
    }
}

/// Extracts all doc string attributes from the list and returns them as list of
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


fn extract_config_attrs(attrs: &mut Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    extract_attrs(attrs, |attr| {
        if attr.path.is_ident("config") {
            // TODO: clone not necessary once we use drain_filter
            Some(attr.clone())
        } else {
            None
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
