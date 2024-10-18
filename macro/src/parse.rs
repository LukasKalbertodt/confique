use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
use syn::{Error, Token, parse::{Parse, ParseStream}, spanned::Spanned, punctuated::Punctuated};

use crate::{
    ir::{Expr, Field, FieldKind, FieldValidator, Input, LeafKind, MapEntry, MapKey},
    util::{is_option, unwrap_option},
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
            validate: attrs.validate,
            name: input.ident,
            fields,
        })
    }
}

// ===== Attributes on the struct =====================================================

#[derive(Default)]
struct StructAttrs {
    partial_attrs: Vec<TokenStream>,
    validate: Option<syn::Path>,
}

enum StructAttr {
    PartialAttrs(TokenStream),
    Validate(syn::Path),
}

impl StructAttrs {
    fn extract(attrs: &mut Vec<syn::Attribute>) -> Result<Self, Error> {
        let attrs = extract_config_attrs(attrs);

        let mut out = Self::default();
        for attr in attrs {
            type AttrList = Punctuated<StructAttr, Token![,]>;
            let parsed_list = attr.parse_args_with(AttrList::parse_terminated)?;

            for parsed in parsed_list {
                let keyword = parsed.keyword();

                macro_rules! duplicate_if {
                    ($cond:expr) => {
                        if $cond {
                            let msg = format!("duplicate '{keyword}' confique attribute");
                            return Err(Error::new(attr.path().span(), msg));
                        }
                    };
                }

                match parsed {
                    StructAttr::PartialAttrs(tokens) => out.partial_attrs.push(tokens),
                    StructAttr::Validate(path) => {
                        duplicate_if!(out.validate.is_some());
                        out.validate = Some(path);
                    }
                }
            }
        }

        Ok(out)
    }
}

impl StructAttr {
    fn keyword(&self) -> &'static str {
        match self {
            Self::PartialAttrs(_) => "partial_attr",
            Self::Validate(_) => "validate",
        }
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
            "validate" => parse_eq_value(input).map(Self::Validate),
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

            let conflicting_attrs = [
                ("default", attrs.default.is_some()),
                ("env", attrs.env.is_some()),
                ("deserialize_with", attrs.deserialize_with.is_some()),
                ("validate", attrs.validate.is_some()),
            ];

            for (keyword, is_set) in conflicting_attrs {
                if is_set {
                    return Err(Error::new(
                        field.ident.span(),
                        format!("cannot specify `nested` and `{keyword}` \
                            attributes at the same time")
                    ));
                }
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
                validate: attrs.validate,
                kind,
            }
        };

        Ok(Self {
            doc,
            name: field.ident.expect("bug: expected named field"),
            kind,
        })
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
    validate: Option<FieldValidator>,
}

enum FieldAttr {
    Nested,
    Default(Expr),
    Env(String),
    DeserializeWith(syn::Path),
    ParseEnv(syn::Path),
    Validate(FieldValidator),
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
                            return Err(Error::new(attr.path().span(), msg));
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
                    FieldAttr::Validate(path) => {
                        duplicate_if!(out.validate.is_some());
                        out.validate = Some(path);
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
            Self::Validate(_) => "validate",
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

            "default" => parse_eq_value(input).map(Self::Default),

            "env" => {
                let key: syn::LitStr = parse_eq_value(input)?;
                let value = key.value();
                if value.contains('=') || value.contains('\0') {
                    return Err(syn::Error::new(
                        key.span(),
                        "environment variable key must not contain '=' or null bytes",
                    ));
                }

                Ok(Self::Env(value))
            }

            "parse_env" => parse_eq_value(input).map(Self::ParseEnv),
            "deserialize_with" => parse_eq_value(input).map(Self::DeserializeWith),
            "validate" => {
                if input.peek(Token![=]) {
                    parse_eq_value(input).map(|path| Self::Validate(FieldValidator::Fn(path)))
                } else if input.peek(syn::token::Paren) {
                    let g: Group = input.parse()?;

                    // Instead of properly parsing an expression, which would
                    // require the `full` feature of syn, increasing compile
                    // time, we just validate the last two/three tokens and
                    // just assume the tokens before are a valid expression.
                    let mut tokens = g.stream().into_iter().collect::<Vec<_>>();
                    if tokens.len() < 3 {
                        return Err(syn::Error::new(
                            g.span(),
                            "expected at least three tokens, found fewer",
                        ));
                    }

                    // Ignore trailing comma
                    if is_comma(tokens.last().unwrap()) {
                        let _ = tokens.pop();
                    }

                    let msg = as_string_lit(tokens.pop().unwrap())?;
                    let sep_comma = tokens.pop().unwrap();
                    if !is_comma(&sep_comma) {
                        return Err(syn::Error::new(sep_comma.span(), "expected comma"));
                    }

                    Ok(Self::Validate(FieldValidator::Simple(tokens.into_iter().collect(), msg)))
                } else {
                    Err(syn::Error::new(
                        ident.span(),
                        "expected `validate = path::to::fun` or `validate(<expr>, \"error msg\")`, \
                            but found different token",
                    ))
                }
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

fn is_comma(tt: &TokenTree) -> bool {
    matches!(tt, TokenTree::Punct(p) if p.as_char() == ',')
}

fn as_string_lit(tt: TokenTree) -> Result<String, syn::Error> {
    let lit = match tt {
        TokenTree::Literal(lit) => syn::Lit::new(lit),
        t => return Err(syn::Error::new(t.span(), "expected string literal")),
    };
    match lit {
        syn::Lit::Str(s) => Ok(s.value()),
        l => return Err(syn::Error::new(l.span(), "expected string literal")),
    }
}

/// Parses a `=` followed by `T`, and asserts that the input is either empty or
/// a comma follows.
fn parse_eq_value<T: syn::parse::Parse>(input: ParseStream) -> Result<T, Error> {
    let _: Token![=] = input.parse()?;
    let out: T = input.parse()?;
    assert_empty_or_comma(&input)?;
    Ok(out)
}

/// Extracts all doc string attributes from the list and returns them as list of
/// strings (in order).
fn extract_doc(attrs: &mut Vec<syn::Attribute>) -> Vec<String> {
    extract_attrs(attrs, |attr| {
        match &attr.meta {
            syn::Meta::NameValue(syn::MetaNameValue {
                value: syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }),
                path,
                ..
            }) if path.is_ident("doc") => Some(s.value()),
            _ => None,
        }
    })
}


fn extract_config_attrs(attrs: &mut Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    extract_attrs(attrs, |attr| {
        if attr.path().is_ident("config") {
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
