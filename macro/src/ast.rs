//! Definition of the intermediate representation or AST.

use proc_macro2::{Ident, TokenStream};


/// The parsed input to the `gen_config` macro.
pub(crate) struct Input {
    pub(crate) root: Node,
    pub(crate) visibility: Option<TokenStream>,
}

/// One node in the tree of the configuration format. Can either be a leaf node
/// (a string, int, float or bool value) or an internal node that contains
/// children.
pub(crate) enum NodeKind {
    Obj(Obj),
    Leaf(Leaf),
}

pub(crate) struct Node {
    /// The doc string lines.
    pub(crate) doc: Vec<String>,
    /// Attributes that are used as specified and not interpreted by us.
    pub(crate) attrs: Vec<syn::Attribute>,

    pub(crate) name: syn::Ident,
    pub(crate) kind: NodeKind,
}

pub(crate) struct Obj {
    pub(crate) typename: Option<syn::Ident>,
    pub(crate) children: Vec<Node>,
}

pub(crate) struct Leaf {
    pub(crate) ty: syn::Type,
    pub(crate) default: Option<Expr>,
    pub(crate) example: Option<Expr>,
}

/// The kinds of expressions (just literals) we allow for default or example
/// values.
pub(crate) enum Expr {
    Str(syn::LitStr),
    Int(syn::LitInt),
    Float(syn::LitFloat),
    Bool(syn::LitBool),
}

impl Node {
    pub(crate) fn typename(&self) -> Option<syn::Ident> {
        match &self.kind {
            NodeKind::Obj(Obj { typename, .. }) => {
                use heck::CamelCase;

                let out = typename.clone().unwrap_or_else(|| {
                    Ident::new(&self.name.to_string().to_camel_case(), self.name.span())
                });

                Some(out)
            }
            NodeKind::Leaf(_) => None,
        }
    }
}
