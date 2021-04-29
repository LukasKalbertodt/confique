//! Definition of the intermediate representation or AST.

/// The parsed input to the `gen_config` macro.
pub(crate) struct Input {
    pub(crate) root: Node,
}

/// One node in the tree of the configuration format. Can either be a leaf node
/// (a string, int, float or bool value) or an internal node that contains
/// children.
pub(crate) enum Node {
    Internal {
        doc: Vec<String>,
        name: syn::Ident,
        children: Vec<Node>,
    },
    Leaf {
        doc: Vec<String>,
        name: syn::Ident,
        ty: syn::Type,
        default: Option<Expr>,
        example: Option<Expr>,
    },
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
    pub(crate) fn name(&self) -> &syn::Ident {
        match self {
            Self::Internal { name, .. } => name,
            Self::Leaf { name, .. } => name,
        }
    }
}
