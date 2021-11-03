use darling::Error;
use syn::spanned::Spanned;

use crate::{ir, util::{is_option, unwrap_option}};


macro_rules! bail {
    ($span:expr, $msg:expr $(,)?) => {
        return Err(Error::custom($msg).with_span(&$span))
    };
}

impl ir::Input {
    pub(crate) fn from_ast(mut input: syn::DeriveInput) -> Result<Self, Error> {
        let struct_fields = match input.data {
            syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(f), .. }) => f,
            _ => bail!(
                input.span(),
                "`confique::Config` can only be derive for structs with named fields",
            ),
        };

        let doc = extract_doc(&mut input.attrs);

        let mut errors = Vec::new();
        let mut fields = Vec::new();
        for field in struct_fields.named {
            match ir::Field::from_ast(field) {
                Ok(f) => fields.push(f),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            return Err(Error::multiple(errors));
        }

        Ok(Self {
            doc,
            visibility: input.vis,
            name: input.ident,
            fields,
        })
    }
}

impl ir::Field {
    fn from_ast(field: syn::Field) -> Result<Self, darling::Error> {
        use darling::FromField;
        let mut field = Field::from_field(&field)?;
        let doc = extract_doc(&mut field.attrs);

        let kind = if field.nested {
            // Nested field.

            if is_option(&field.ty) {
                bail!(
                    field.ident.span(),
                    "nested configurations cannot be optional (type `Option<_>`)",
                );
            }
            if field.default.is_some() {
                bail!(
                    field.ident.span(),
                    "cannot specify `nested` and `default` attributes at the same time",
                );
            }
            if field.env.is_some() {
                bail!(
                    field.ident.span(),
                    "cannot specify `nested` and `env` attributes at the same time",
                );
            }

            ir::FieldKind::Nested { ty: field.ty }
        } else {
            // Leaf field.

            match unwrap_option(&field.ty) {
                None => ir::FieldKind::Leaf {
                    env: field.env,
                    kind: ir::LeafKind::Required {
                        default: field.default,
                        ty: field.ty,
                    },
                },
                Some(inner) => {
                    if field.default.is_some() {
                        bail!(
                            field.ident.span(),
                            "optional fields (type `Option<_>`) cannot have default \
                                values (`#[config(default = ...)]`)",
                        );
                    }

                    ir::FieldKind::Leaf {
                        env: field.env,
                        kind: ir::LeafKind::Optional {
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
        matches!(self.kind, ir::FieldKind::Leaf { .. })
    }
}

impl darling::FromMeta for ir::Expr {
    fn from_value(lit: &syn::Lit) -> Result<Self, darling::Error> {
        match lit {
            syn::Lit::Str(l) => Ok(Self::Str(l.clone())),
            syn::Lit::Int(l) => Ok(Self::Int(l.clone())),
            syn::Lit::Float(l) => Ok(Self::Float(l.clone())),
            syn::Lit::Bool(l) => Ok(Self::Bool(l.clone())),

            _ => {
                // let msg = "only string, integer, float and bool literals are allowed here";
                // Err(Error::new(lit.span(), msg))
                Err(darling::Error::unexpected_lit_type(lit))
            }
        }
    }
}



#[derive(Debug, darling::FromField)]
#[darling(attributes(config), forward_attrs(doc))]
struct Field {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    attrs: Vec<syn::Attribute>,

    #[darling(default)]
    nested: bool,

    #[darling(default)]
    env: Option<String>,

    #[darling(default)]
    default: Option<ir::Expr>,
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
