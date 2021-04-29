use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;
use std::fmt::{self, Write};

use crate::ast::{Expr, Input, Leaf, Node, NodeKind, Obj};


pub(crate) fn gen(input: Input) -> TokenStream {
    let visibility = input.visibility.clone().unwrap_or(quote! { pub(crate) });
    let toml = gen_toml(&input);
    let types = gen_types(&input, &visibility);

    quote! {
        #visibility const TOML_TEMPLATE: &str = #toml;

        #types
    }
}

/// Generates the struct fields for both, the raw struct and the main struct.
fn gen_struct_fields(children: &[Node], visibility: &TokenStream) -> (TokenStream, TokenStream) {
    let mut raw_fields = TokenStream::new();
    let mut main_fields = TokenStream::new();

    for child in children {
        let name = &child.name;
        let doc = &child.doc;

        match &child.kind {
            NodeKind::Obj(_) => {
                let child_typename = child.typename().unwrap();
                let default_path = format!("{}::empty", child_typename);
                raw_fields.extend(quote! {
                    #[serde(default = #default_path)]
                    #visibility #name: #child_typename,
                });
                main_fields.extend(quote! {
                    #visibility #name: #child_typename,
                });
            }
            NodeKind::Leaf(Leaf { ty, .. }) => {
                let inner = as_option(&ty).unwrap_or(&ty);
                raw_fields.extend(quote! {
                    #visibility #name: Option<#inner>,
                });
                main_fields.extend(quote! {
                    #( #[doc = #doc] )*
                    #visibility #name: #ty,
                });
            }
        }
    }

    (raw_fields, main_fields)
}

/// Generates the definition for `default_values`, a function associated with raw types.
fn gen_raw_default_constructor(
    children: &[Node],
    path: &[String],
    visibility: &TokenStream,
) -> TokenStream {
    let fields = collect_tokens(children, |node| {
        let name = &node.name;
        match &node.kind {
            NodeKind::Leaf(Leaf { default: None, .. }) => quote! { #name: None, },
            NodeKind::Leaf(Leaf { default: Some(expr), ty, .. }) => {
                // TODO: we can specialize this for certain types such that we
                // don't have to invoke serde.
                let inner_type = as_option(ty).unwrap_or(ty);
                let path = append_path(path, name);
                let msg = format!(
                    "default configuration value for '{}' cannot be deserialized as '{}'",
                    path,
                    inner_type.to_token_stream(),
                );

                quote! {
                    #name: Some({
                        let result: Result<_, confique::serde::de::value::Error>
                            = Deserialize::deserialize(#expr.into_deserializer());
                        result.expect(#msg)
                    }),
                }
            },
            NodeKind::Obj(_) => {
                let child_typename = node.typename().unwrap();
                quote! {
                    #name: #child_typename::default_values(),
                }
            }
        }
    });

    quote! {
        /// Returns an instance of `Self` that contains the specified default
        /// configuration values. All fields that don't have a default value
        /// specified are `None`.
        #visibility fn default_values() -> Self {
            Self { #fields }
        }
    }
}

/// Generates the definition for `empty`, a function associated with raw types.
fn gen_raw_empty_constructor(children: &[Node], visibility: &TokenStream) -> TokenStream {
    let fields = collect_tokens(children, |node| {
        let name = &node.name;
        match &node.kind {
            NodeKind::Leaf(_) => quote! { #name: None, },
            NodeKind::Obj(_) => {
                let child_typename = node.typename().unwrap();
                quote! {
                    #name: #child_typename::empty(),
                }
            }
        }
    });

    quote! {
        /// Returns an instance of `Self` where all values are `None`.
        #visibility fn empty() -> Self {
            Self { #fields }
        }
    }
}

/// Generates the definition of the `overwrite_with` method on raw types.
fn gen_raw_overwrite_with_method(children: &[Node], visibility: &TokenStream) -> TokenStream {
    let fields = collect_tokens(children, |Node { name, kind, .. }| {
        match kind {
            NodeKind::Leaf(_) => quote! { #name: other.#name.or(self.#name), },
            NodeKind::Obj(_) => quote! { #name: self.#name.overwrite_with(other.#name), },
        }
    });

    quote! {
        // TODO: Find better name
        #visibility fn overwrite_with(self, other: Self) -> Self {
            Self { #fields }
        }
    }
}

/// Generates the impl to convert from a raw type to a main type.
fn gen_try_from_impl(typename: &Ident, children: &[Node], path: &[String]) -> TokenStream {
    let fields = collect_tokens(children, |Node { name, kind, .. }| {
        match kind {
            NodeKind::Leaf(Leaf { ty, .. }) => {
                if as_option(ty).is_some() {
                    // If this value is optional, we just move it as it can never fail.
                    quote! { #name: src.#name, }
                } else {
                    // Otherwise, we return an error if the value hasn't been specified.
                    let path = append_path(path, name);

                    quote! {
                        #name: src.#name.ok_or(confique::TryFromError { path: #path })?,
                    }
                }
            },
            NodeKind::Obj(_) => quote! {
                #name: std::convert::TryFrom::try_from(src.#name)?,
            },
        }
    });

    quote! {
        impl std::convert::TryFrom<raw::#typename> for #typename {
            type Error = confique::TryFromError;
            fn try_from(src: raw::#typename) -> Result<Self, Self::Error> {
                Ok(Self {
                    #fields
                })
            }
        }
    }
}

fn append_path(path: &[String], name: &Ident) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", path.join("."), name)
    }
}

fn gen_types(input: &Input, visibility: &TokenStream) -> TokenStream {
    let mut raw_types = TokenStream::new();
    let mut main_types = TokenStream::new();

    visit(input, |node, path| {
        if let NodeKind::Obj(Obj { children, .. }) = &node.kind {
            let typename = node.typename().unwrap();

            let (raw_fields, main_fields) = gen_struct_fields(&children, visibility);
            let raw_default_constructor = gen_raw_default_constructor(&children, path, visibility);
            let raw_empty_constructor = gen_raw_empty_constructor(&children, visibility);
            let overwrite_with_method = gen_raw_overwrite_with_method(&children, visibility);
            let try_from_impl = gen_try_from_impl(&typename, &children, path);

            // Raw type definition
            raw_types.extend(quote! {
                #[derive(Debug, Deserialize)]
                #visibility struct #typename {
                    #raw_fields
                }

                impl #typename {
                    #raw_default_constructor
                    #raw_empty_constructor
                    #overwrite_with_method
                }
            });

            // Main type definition
            let doc = &node.doc;
            let attrs = &node.attrs;
            let derives = input.derive_for_all.clone().unwrap_or(quote! { Debug });

            main_types.extend(quote! {
                #( #[doc = #doc] )*
                #( #attrs )*
                #[derive( #derives )]
                #visibility struct #typename {
                    #main_fields
                }

                #try_from_impl
            });
        }
    });

    quote! {
        /// Types where all configuration values are optional.
        ///
        /// The types in this module also represent the full configuration tree,
        /// but all values are optional. That's useful for intermediate steps or
        /// "layers" of configuration sources. Imagine that the three layers:
        /// environment variables, a TOML file and the fixed default values. The
        /// only thing that matters is that required values are present after
        /// merging all sources, but each individual is allowed to lack required
        /// values.
        ///
        /// These types implement `serde::Deserialize` and `Debug`.
        #visibility mod raw {
            // We have to add this blanket use to be able to refer to all the
            // types the user referred to.
            use super::*;

            use confique::serde::{Deserialize, de::IntoDeserializer};

            #raw_types
        }

        #main_types
    }
}


/// Generates the TOML template file.
fn gen_toml(input: &Input) -> String {
    /// Writes all doc comments to the file.
    fn write_doc(out: &mut String, doc: &[String]) {
        for line in doc {
            writeln!(out, "#{}", line).unwrap();
        }
    }

    /// Adds zero, one or two line breaks to make sure that there are at least
    /// two line breaks at the end of the string.
    fn add_empty_line(out: &mut String) {
        match () {
            () if out.ends_with("\n\n") => {},
            () if out.ends_with('\n') => out.push('\n'),
            _ => out.push_str("\n\n"),
        }
    }


    let mut out = String::new();
    visit(input, |Node { name, doc, kind, .. }, path| {
        match kind {
            NodeKind::Obj(_) => {
                write_doc(&mut out, doc);

                // If a new subsection starts, we always print the header, even if not
                // strictly necessary.
                if path.is_empty() {
                    add_empty_line(&mut out);
                } else {
                    writeln!(out, "[{}]", path.join(".")).unwrap();
                }
            }

            NodeKind::Leaf(Leaf { ty, default, example }) => {
                write_doc(&mut out, doc);

                // Add note about default value or the value being required.
                match default {
                    Some(default) => {
                        if !doc.is_empty() {
                            writeln!(out, "#").unwrap();
                        }
                        writeln!(out, "# Default: {}", default).unwrap();
                    }
                    None if as_option(ty).is_some() => {}
                    None => {
                        if !doc.is_empty() {
                            writeln!(out, "#").unwrap();
                        }
                        writeln!(out, "# Required! This value must be specified.").unwrap();
                    }
                }

                // We check that already when parsing.
                let example = example.as_ref()
                    .or(default.as_ref())
                    .expect("neither example nor default");

                // Commented out example.
                writeln!(out, "#{} = {}", name, example).unwrap();
                add_empty_line(&mut out);
            }
        }
    });

    // Make sure there is only a single trailing newline.
    while out.ends_with("\n\n") {
        out.pop();
    }

    out
}

/// Visits all nodes in depth-first session (visiting the parent before its
/// children).
fn visit<F>(input: &Input, mut visitor: F)
where
    F: FnMut(&Node, &[String]),
{
    let mut stack = vec![(&input.root, vec![])];
    while let Some((node, path)) = stack.pop() {
        visitor(&node, &path);

        if let NodeKind::Obj(Obj { children, .. }) = &node.kind {
            for child in children.iter().rev() {
                let mut child_path = path.clone();
                child_path.push(child.name.to_string());
                stack.push((child, child_path));
            }
        }
    }
}

/// Iterates over `it`, calling `f` for each element, collecting all returned
/// token streams into one.
fn collect_tokens<T>(
    it: impl IntoIterator<Item = T>,
    f: impl FnMut(T) -> TokenStream,
) -> TokenStream {
    it.into_iter().map(f).collect()
}

/// Checks if the given type is an `Option` and if so, return the inner type.
///
/// Note: this function clearly shows one of the major shortcomings of proc
/// macros right now: we do not have access to the compiler's type tables and
/// can only check if it "looks" like an `Option`. Of course, stuff can go
/// wrong. But that's the best we can do and it's highly unlikely that someone
/// shadows `Option`.
fn as_option(ty: &syn::Type) -> Option<&syn::Type> {
    let ty = match ty {
        syn::Type::Path(path) => path,
        _ => return None,
    };

    if ty.qself.is_some() || ty.path.leading_colon.is_some() {
        return None;
    }

    let valid_paths = [
        &["Option"] as &[_],
        &["std", "option", "Option"],
        &["core", "option", "Option"],
    ];
    if !valid_paths.iter().any(|vp| ty.path.segments.iter().map(|s| &s.ident).eq(*vp)) {
        return None;
    }

    let args = match &ty.path.segments.last().unwrap().arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => return None,
    };

    if args.args.len() != 1 {
        return None;
    }

    match &args.args[0] {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}


impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Str(lit) => lit.to_tokens(tokens),
            Self::Int(lit) => lit.to_tokens(tokens),
            Self::Float(lit) => lit.to_tokens(tokens),
            Self::Bool(lit) => lit.to_tokens(tokens),
        }
    }
}

// This `Display` impl is for writing into a TOML file.
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // TODO: not sure if `escape_debug` is really what we want here, but
            // it's working for now.
            Self::Str(lit) => write!(f, "\"{}\"", lit.value().escape_debug()),
            Self::Int(lit) => lit.fmt(f),
            Self::Float(lit) => lit.fmt(f),
            Self::Bool(lit) => lit.value.fmt(f),
        }
    }
}
