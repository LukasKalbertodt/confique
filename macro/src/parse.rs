use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    Error, Token,
};

use crate::{
    ir::{
        Expr, Field, FieldKind, FieldValidator, FloatType, Input, IntType, LeafKind, MapEntry,
        MapKey,
    },
    scan::ExprScanner,
    util::{get_array_item_type, get_map_entry_types, is_option, unwrap_option},
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
            layer_attrs: attrs.layer_attrs,
            validate: attrs.validate,
            name: input.ident,
            layer_name: attrs.layer_name,
            fields,
        })
    }
}

// ===== Attributes on the struct =====================================================

#[derive(Default)]
struct StructAttrs {
    layer_attrs: Vec<TokenStream>,
    validate: Option<syn::Path>,
    layer_name: Option<Ident>,
}

enum StructAttr {
    LayerAttr(TokenStream),
    Validate(syn::Path),
    LayerName(Ident),
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
                    StructAttr::LayerAttr(tokens) => out.layer_attrs.push(tokens),
                    StructAttr::Validate(path) => {
                        duplicate_if!(out.validate.is_some());
                        out.validate = Some(path);
                    }
                    StructAttr::LayerName(name) => {
                        duplicate_if!(out.layer_name.is_some());
                        out.layer_name = Some(name);
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
            Self::LayerAttr(_) => "layer_attr",
            Self::Validate(_) => "validate",
            Self::LayerName(_) => "layer_name",
        }
    }
}

impl Parse for StructAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match &*ident.to_string() {
            "layer_attr" => {
                let stream = parse_layer_attr(input)?;
                Ok(Self::LayerAttr(stream))
            }
            "validate" => parse_eq_value(input).map(Self::Validate),
            "layer_name" => parse_eq_value(input).map(Self::LayerName),
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
                ("parse_env", attrs.parse_env.is_some()),
                ("deserialize_with", attrs.deserialize_with.is_some()),
                ("validate", !attrs.validate.is_empty()),
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
                None => {
                    let mut default = attrs.default;
                    if let Some(default) = &mut default {
                        default.infer_types(Some(&field.ty));
                    }
                    LeafKind::Required { default, ty: field.ty }
                }
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
            layer_attrs: attrs.layer_attrs
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
    validate: Vec<FieldValidator>,
    layer_attrs: Vec<TokenStream>,
}

enum FieldAttr {
    Nested,
    Default(Expr),
    Env(String),
    DeserializeWith(syn::Path),
    ParseEnv(syn::Path),
    Validate(FieldValidator),
    LayerAttr(TokenStream),
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
                    FieldAttr::Validate(validator) => {
                        out.validate.push(validator);
                    }
                    FieldAttr::LayerAttr(layer_attr) => {
                        out.layer_attrs.push(layer_attr);
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
            Self::LayerAttr(_) => "layer_attr",
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
                let expr = Expr::parse_default(input)?;
                assert_empty_or_comma(input)?;
                Ok(Self::Default(expr))
            }

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
            "layer_attr" => {
                let layer_stream = parse_layer_attr(input)?;
                Ok(Self::LayerAttr(layer_stream))
            }

            _ => Err(syn::Error::new(ident.span(), "unknown confique attribute")),
        }
    }
}


// ===== Expr =====================================================================

impl Expr {
    /// Parses a default value, i.e. only literals, arrays and maps are allowed.
    pub(crate) fn parse_default(input: ParseStream) -> Result<Self, Error> {
        Self::parse_value(input, false)
    }

    /// Parses a value of `layer!`: like a default value, but with arbitrary
    /// Rust expressions instead of just literals.
    pub(crate) fn parse_with_rust(input: ParseStream) -> Result<Self, Error> {
        Self::parse_value(input, true)
    }

    fn parse_value(input: ParseStream, allow_rust: bool) -> Result<Self, Error> {
        // If Rust expressions are allowed, a `[...]` or `{...}` is only an
        // array or map if it is the whole value. For example, `[1, 2].len()`
        // is a Rust expression. So is a `{...}` not looking like a map, which
        // is a Rust block, and `[x; 3]`, which has a `;` directly inside
        // the brackets.
        let fork = input.fork();
        let group = fork.parse::<Group>().ok();
        let whole_value = group.is_some() && at_value_end(&fork);
        let is_bracket = input.peek(syn::token::Bracket);
        let has_semicolon = group.into_iter().flat_map(|g| g.stream())
            .any(|tt| matches!(&tt, TokenTree::Punct(p) if p.as_char() == ';'));
        let is_array_repeat = allow_rust && is_bracket && whole_value && has_semicolon;
        let is_array = is_bracket && (!allow_rust || whole_value) && !is_array_repeat;
        let is_map = input.peek(syn::token::Brace)
            && (!allow_rust || (whole_value && looks_like_map(input)));

        if is_array {
            let content;
            syn::bracketed!(content in input);
            parse_list(&content, |input| Self::parse_value(input, allow_rust)).map(Self::Array)
        } else if is_map {
            let content;
            syn::braced!(content in input);
            parse_list(&content, |input| MapEntry::parse(input, allow_rust)).map(Self::Map)
        } else if is_array_repeat {
            // This becomes a `Vec`, as arrays are not deserializable.
            let group: Group = input.parse()?;
            Ok(Self::Rust(quote::quote! { std::vec! #group }))
        } else if allow_rust {
            parse_rust_expr(input).map(Self::Rust)
        } else {
            let msg = "invalid default value. Allowed are only: certain literals \
                (string, integer, float, bool), arrays and maps.";

            // We just use `MapKey` here as it's exactly what we want, despite
            // this not having anything to do with maps.
            input.parse::<MapKey>()
                .map_err(|_| Error::new(input.span(), msg))
                .map(Into::into)
        }
    }
}

impl MapEntry {
    fn parse(input: ParseStream, allow_rust: bool) -> Result<Self, Error> {
        let key: MapKey = input.parse()?;
        let _: syn::Token![:] = input.parse()?;
        let value = Expr::parse_value(input, allow_rust)?;
        Ok(Self { key, value })
    }
}

/// Returns the tokens of a Rust expression: everything up to the comma ending
/// it or the end of input. See [`ExprScanner`].
fn parse_rust_expr(input: ParseStream) -> Result<TokenStream, Error> {
    let mut scanner = ExprScanner::default();
    let mut tokens = TokenStream::new();
    while !input.is_empty() && !(scanner.at_top_level() && input.peek(Token![,])) {
        let tt: TokenTree = input.parse()?;
        scanner.push(&tt);
        tokens.extend([tt]);
    }

    if tokens.is_empty() {
        return Err(input.error("expected value"));
    }
    Ok(tokens)
}

/// Returns `true` if the `{...}` at the start of `input` is empty or starts
/// with `<literal>:`.
fn looks_like_map(input: ParseStream) -> bool {
    peek_braces(input, |content| {
        content.is_empty() || (content.parse::<syn::Lit>().is_ok() && peek_colon(content))
    })
}

impl Parse for MapKey {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lit: syn::Lit = input.parse()?;
        match lit {
            syn::Lit::Str(l) => Ok(Self::Str(l)),
            syn::Lit::Int(l) => {
                let ty = IntType::from_suffix(l.suffix());
                Ok(Self::Int(l, ty))
            }
            syn::Lit::Float(l) => {
                let ty = FloatType::from_suffix(l.suffix());
                Ok(Self::Float(l, ty))
            }
            syn::Lit::Bool(l) => Ok(Self::Bool(l)),
            _ => Err(Error::new(
                lit.span(),
                "only string, integer, float, and Boolean literals allowed as map key",
            )),
        }
    }
}

impl Expr {
    /// Infers the types of int and float literals without suffix from the
    /// field type `ty`, e.g. `5` for a field `u64` is `u64`. This is just a
    /// heuristic: if no type can be inferred, they stay `None`.
    fn infer_types(&mut self, ty: Option<&syn::Type>) {
        match self {
            Self::Str(_) | Self::Bool(_) | Self::Rust(_) => {}
            Self::Int(_, int_ty) => infer_num_type(ty, IntType::from_suffix, int_ty),
            Self::Float(_, float_ty) => infer_num_type(ty, FloatType::from_suffix, float_ty),
            Self::Array(items) => {
                let item_ty = ty.and_then(get_array_item_type);
                items.iter_mut().for_each(|item| item.infer_types(item_ty));
            }
            Self::Map(entries) => {
                let types = ty.and_then(get_map_entry_types);
                for entry in entries {
                    entry.key.infer_type(types.map(|(k, _)| k));
                    entry.value.infer_types(types.map(|(_, v)| v));
                }
            }
        }
    }
}

impl MapKey {
    /// Like `Expr::infer_types`.
    fn infer_type(&mut self, ty: Option<&syn::Type>) {
        match self {
            Self::Str(_) | Self::Bool(_) => {}
            Self::Int(_, int_ty) => infer_num_type(ty, IntType::from_suffix, int_ty),
            Self::Float(_, float_ty) => infer_num_type(ty, FloatType::from_suffix, float_ty),
        }
    }
}

/// If `out` is `None` (i.e. the literal has no suffix) and `field_ty` is a
/// number type known to `from_suffix` (e.g. `u64`), sets `out` to that type.
fn infer_num_type<T>(
    field_ty: Option<&syn::Type>,
    from_suffix: fn(&str) -> Option<T>,
    out: &mut Option<T>,
) {
    if out.is_some() {
        return;
    }

    if let Some(syn::Type::Path(syn::TypePath { qself: None, path })) = field_ty {
        *out = path.get_ident().and_then(|i| from_suffix(&i.to_string()));
    }
}


// ===== Util =====================================================================

fn parse_layer_attr(input: &ParseBuffer<'_>) -> syn::Result<TokenStream> {
    let g: Group = input.parse()?;
    if g.delimiter() != Delimiter::Parenthesis {
        return Err(Error::new_spanned(g,
            "expected `(...)` but found different delimiter"));
    }
    assert_empty_or_comma(&input)?;
    Ok(g.stream())
}

/// Parses a comma separated list with optional trailing comma, until `input`
/// is empty.
pub(crate) fn parse_list<T>(
    input: ParseStream,
    parse_item: impl Fn(ParseStream) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let mut out = Vec::new();
    while !input.is_empty() {
        out.push(parse_item(input)?);
        if !input.is_empty() {
            let _: Token![,] = input.parse()?;
        }
    }
    Ok(out)
}

/// Returns `true` if a value ends here: at a comma or the end of input.
pub(crate) fn at_value_end(input: ParseStream) -> bool {
    input.is_empty() || input.peek(Token![,])
}

/// Returns `true` if `input` starts with a single colon, not with `::`.
pub(crate) fn peek_colon(input: ParseStream) -> bool {
    input.peek(Token![:]) && !input.peek(Token![::])
}

/// Applies `f` to the content of the `{...}` at the start of `input`, without
/// advancing `input`. Returns `false` if there is no `{...}`.
pub(crate) fn peek_braces(input: ParseStream, f: fn(ParseStream) -> bool) -> bool {
    let group = match input.fork().parse::<Group>() {
        Ok(group) if group.delimiter() == Delimiter::Brace => group,
        _ => return false,
    };

    // `f` usually only looks at the first few tokens, but `parse2` errors if
    // there are tokens left. So we consume the rest.
    let parser = |content: ParseStream| {
        let out = f(content);
        let _: TokenStream = content.parse()?;
        Ok(out)
    };
    parser.parse2(group.stream()).unwrap_or(false)
}

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
