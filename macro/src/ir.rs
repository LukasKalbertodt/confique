//! Definition of the intermediate representation.

use syn::{Error, Token, parse::{Parse, ParseStream}, spanned::Spanned};


/// The parsed input to the `gen_config` macro.
#[derive(Debug)]
pub(crate) struct Input {
    pub(crate) doc: Vec<String>,
    pub(crate) visibility: syn::Visibility,
    pub(crate) name: syn::Ident,
    pub(crate) fields: Vec<Field>,
}

#[derive(Debug)]
pub(crate) struct Field {
    pub(crate) doc: Vec<String>,
    pub(crate) name: syn::Ident,
    pub(crate) ty: syn::Type,
    pub(crate) default: Option<Expr>,

    // TODO:
    // - serde attributes
    // - attributes
    // - example
}

/// The kinds of expressions (just literals) we allow for default or example
/// values.
#[derive(Debug)]
pub(crate) enum Expr {
    Str(syn::LitStr),
    Int(syn::LitInt),
    Float(syn::LitFloat),
    Bool(syn::LitBool),
    // TODO: arrays?
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

        Ok(Self {
            doc,
            default: attrs.default,
            name: field.ident.expect("bug: expected named field"),
            ty: field.ty,
        })
    }
}

impl Expr {
    fn from_lit(lit: syn::Lit) -> Result<Self, Error> {
        match lit {
            syn::Lit::Str(l) => Ok(Self::Str(l)),
            syn::Lit::Int(l) => Ok(Self::Int(l)),
            syn::Lit::Float(l) => Ok(Self::Float(l)),
            syn::Lit::Bool(l) => Ok(Self::Bool(l)),

            _ => {
                let msg = "only string, integer, float and bool literals are allowed here";
                Err(Error::new(lit.span(), msg))
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
        match attr.parse_args::<InternalAttr>()? {
            InternalAttr::Default(expr) => {
                if out.default.is_some() {
                    let msg = format!(
                        "duplicate '{}' confique attribute",
                        attr.path.get_ident().unwrap()
                    );
                    return Err(Error::new(attr.span(), msg));
                }

                out.default = Some(expr);
            }
        }
    }

    Ok(out)
}

#[derive(Default)]
struct InternalAttrs {
    default: Option<Expr>,
}

enum InternalAttr {
    Default(Expr),
}

impl Parse for InternalAttr {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident: syn::Ident = input.parse()?;
        match &*ident.to_string() {
            "default" => {
                let _: Token![=] = input.parse()?;
                let expr = Expr::from_lit(input.parse()?)?;
                assert_empty(input)?;
                Ok(Self::Default(expr))
            }

            _ => Err(syn::Error::new(ident.span(), "unknown confique attribute")),
        }
    }
}

fn assert_empty(input: ParseStream) -> Result<(), Error> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(Error::new(input.span(), "unexpected tokens, expected no more tokens in this context"))
    }
}
