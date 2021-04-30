use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Error, Ident,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

use crate::ast::{Expr, Input, Leaf, Node, NodeKind, Obj};



impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut outer_attrs = input.call(syn::Attribute::parse_inner)?;
        let visibility = extract_visibility(&mut outer_attrs)?;
        let derive_for_all = extract_single_list_attr("derive_for_all", &mut outer_attrs)?;
        assert_no_extra_attrs(&outer_attrs)?;

        // Parse top level object.
        let root: Node = input.parse()?;
        if root.name != "config" {
            return Err(syn::Error::new(
                root.name.span(),
                "top level object must have the name 'config'",
            ));
        }

        // Make sure we have at most one trailing comma
        if input.peek(syn::Token![,]) {
            let _: syn::Token![,] = input.parse().unwrap();
        }

        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "unexpected additional tokens (only one root element allowed)",
            ));
        }

        Ok(Self { root, visibility, derive_for_all })
    }
}


impl Parse for Node {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut attrs = input.call(syn::Attribute::parse_outer)?;
        let doc = extract_doc(&mut attrs)?;

        // All nodes start with an identifier and a colon.
        let name = input.parse()?;
        let _: syn::Token![:] = input.parse()?;

        let out = if input.lookahead1().peek(syn::token::Brace) {
            // --- A nested Internal ---
            let typename = extract_typename(&mut attrs)?;
            let forwarded_attrs = extract_attrs(&["derive"], &mut attrs);

            let inner;
            syn::braced!(inner in input);
            let fields = inner.call(<Punctuated<_, syn::Token![,]>>::parse_terminated)?;

            Self {
                doc,
                attrs: forwarded_attrs,
                name,
                kind: NodeKind::Obj(Obj {
                    typename,
                    children: fields.into_iter().collect(),
                }),
            }
        } else {
            // --- A single value ---

            // Type is mandatory.
            let ty = input.parse()?;

            // Optional default value.
            let default = if input.lookahead1().peek(syn::Token![=]) {
                let _: syn::Token![=] = input.parse()?;
                Some(input.parse()?)
            } else {
                None
            };

            // Optional example value.
            let example = attrs.iter()
                .position(|attr| attr.path.is_ident("example"))
                .map(|i| {
                    let attr = attrs.remove(i);
                    parse_attr_value::<Expr>(attr.tokens)
                })
                .transpose()?;

            Self {
                doc,
                attrs: vec![],
                name,
                kind: NodeKind::Leaf(Leaf {  ty, default, example }),
            }
        };

        assert_no_extra_attrs(&attrs)?;

        Ok(out)
    }
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lit = input.parse::<syn::Lit>()?;
        let out = match lit {
            syn::Lit::Str(l) => Self::Str(l),
            syn::Lit::Int(l) => Self::Int(l),
            syn::Lit::Float(l) => Self::Float(l),
            syn::Lit::Bool(l) => Self::Bool(l),

            _ => {
                let msg = "only string, integer, float and bool literals are allowed here";
                return Err(Error::new(lit.span(), msg));
            }
        };

        Ok(out)
    }
}

/// Makes sure that the given list is empty or returns an error otherwise.
fn assert_no_extra_attrs(attrs: &[syn::Attribute]) -> Result<(), Error> {
    if let Some(attr) = attrs.get(0) {
        let msg = "unknown/unexpected/duplicate attribute in this position";
        return Err(Error::new(attr.span(), msg));
    }

    Ok(())
}

/// Parses the tokenstream as a `T` preceeded by a `=`. This is useful for
/// attributes of the form `#[foo = <T>]`.
fn parse_attr_value<T: Parse>(tokens: TokenStream) -> Result<T, Error> {
    use syn::parse::Parser;

    fn parser<T: Parse>(input: ParseStream) -> Result<T, Error> {
        let _: syn::Token![=] = input.parse()?;
        input.parse()
    }

    parser.parse2(tokens)
}

/// Extract all doc attributes from the list and return them as simple strings.
fn extract_doc(attrs: &mut Vec<syn::Attribute>) -> Result<Vec<String>, Error> {
    let out = attrs.iter()
        .filter(|attr| attr.path.is_ident("doc"))
        .map(|attr| parse_attr_value::<syn::LitStr>(attr.tokens.clone()).map(|lit| lit.value()))
        .collect::<Result<_, _>>()?;

    // I know this is algorithmically not optimal, but `drain_filter` is still
    // unstable and I can't be bothered to write the proper algorithm right now.
    attrs.retain(|attr| !attr.path.is_ident("doc"));

    Ok(out)
}

/// Extracts all attributes with a path contained in `names`.
fn extract_attrs(names: &[&str], attrs: &mut Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    let (matches, rest) = attrs.drain(..)
        .partition(|attr| names.iter().any(|n| attr.path.is_ident(n)));

    *attrs = rest;
    matches
}

fn extract_single_attr(
    name: &str,
    attrs: &mut Vec<syn::Attribute>,
) -> Result<Option<syn::Attribute>, Error> {
    let attr = match attrs.iter().position(|attr| attr.path.is_ident(name)) {
        None => return Ok(None),
        Some(pos) => attrs.remove(pos),
    };

    if let Some(dupe) = attrs.iter().find(|attr| attr.path.is_ident(name)) {
        let msg = format!("duplicate `{}` attribute", name);
        return Err(Error::new(dupe.span(), msg));
    }

    Ok(Some(attr))
}

fn extract_single_name_value_attr(
    name: &str,
    attrs: &mut Vec<syn::Attribute>,
) -> Result<Option<syn::Lit>, Error> {
    let attr = match extract_single_attr(name, attrs)? {
        None => return Ok(None),
        Some(attr) => attr,
    };

    match attr.parse_meta()? {
        syn::Meta::NameValue(nv) => Ok(Some(nv.lit)),
        other => {
            let msg = format!(r#"expected `name = "value"` attribute syntax for `{}`"#, name);
            Err(Error::new(other.span(), msg))
        }
    }
}

fn extract_single_list_attr(
    name: &str,
    attrs: &mut Vec<syn::Attribute>,
) -> Result<Option<TokenStream>, Error> {
    let attr = match extract_single_attr(name, attrs)? {
        None => return Ok(None),
        Some(attr) => attr,
    };

    match attr.parse_meta()? {
        syn::Meta::List(list) => Ok(Some(list.nested.to_token_stream())),
        other => {
            let msg = format!(r#"expected `{}(...)` attribute syntax"#, name);
            return Err(Error::new(other.span(), msg));
        }
    }
}

fn assert_string_lit(lit: syn::Lit) -> Result<String, Error> {
    match lit {
        syn::Lit::Str(s) => Ok(s.value()),
        _ => Err(Error::new(lit.span(), "expected string literal")),
    }
}

/// `#[visibility = "..."]`
fn extract_visibility(attrs: &mut Vec<syn::Attribute>) -> Result<Option<TokenStream>, Error> {
    extract_single_name_value_attr("visibility", attrs)?
        .map(|v| Ok::<_, syn::Error>(assert_string_lit(v)?.parse::<TokenStream>()?))
        .transpose()
}

/// `#[typename = "..."]`
fn extract_typename(attrs: &mut Vec<syn::Attribute>) -> Result<Option<Ident>, Error> {
    extract_single_name_value_attr("typename", attrs)?
        .map(|lit| {
            let span = lit.span();
            let s = assert_string_lit(lit)?;
            Ok(Ident::new(&s, span))
        })
        .transpose()
}
