use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;
use std::fmt::{self, Write};

use crate::ast::{Expr, Input, Node};


pub(crate) fn gen(input: Input) -> TokenStream {
    let visibility = quote! { pub(crate) };
    let toml = gen_toml(&input);
    let root_mod = gen_root_mod(&input, &visibility);
    let raw_mod = gen_raw_mod(&input, &visibility);

    quote! {
        const TOML_TEMPLATE: &str = #toml;

        #root_mod
        #raw_mod
    }
}

fn gen_raw_mod(input: &Input, visibility: &TokenStream) -> TokenStream {
    let mut contents = TokenStream::new();
    visit(input, |node, path| {
        if let Node::Internal { name, children, .. } = node {
            let type_name = to_camel_case(name);

            let raw_fields = collect_tokens(children, |node| {
                match node {
                    Node::Leaf { name, ty, .. } => {
                        let inner = as_option(&ty).unwrap_or(&ty);
                        quote! { #visibility #name: Option<#inner>, }
                    },
                    Node::Internal { name, .. } => {
                        let child_type_name = to_camel_case(name);
                        quote! {
                            #[serde(default)]
                            #visibility #name: #child_type_name,
                        }
                    },
                }
            });

            let default_fields = collect_tokens(children, |node| {
                match node {
                    Node::Leaf { name, default: None, .. } => quote! { #name: None, },
                    Node::Leaf { name, default: Some(expr), ty, .. } => {
                        let inner_type = as_option(ty).unwrap_or(ty);
                        let path = format!("{}.{}", path.join("."), name);
                        let msg = format!(
                            "default configuration value for '{}' cannot be deserialized as '{}'",
                            path,
                            inner_type.to_token_stream(),
                        );

                        quote! {
                            #name: Some({
                                let result: Result<_, ::confique::serde::de::value::Error>
                                    = Deserialize::deserialize(#expr.into_deserializer());
                                result.expect(#msg)
                            }),
                        }
                    },
                    Node::Internal { name, .. } => {
                        let child_type_name = to_camel_case(name);
                        quote! {
                            #name: #child_type_name::default_values(),
                        }
                    }
                }
            });

            let overwrite_with_fields = collect_tokens(children, |node| {
                match node {
                    Node::Leaf { name, .. } => quote! {
                        #name: other.#name.or(self.#name),
                    },
                    Node::Internal { name, .. } => quote! {
                        #name: self.#name.overwrite_with(other.#name),
                    }
                }
            });

            contents.extend(quote! {
                #[derive(Debug, Default, ::confique::serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                #visibility struct #type_name {
                    #raw_fields
                }

                impl #type_name {
                    #visibility fn default_values() -> Self {
                        Self { #default_fields }
                    }

                    #visibility fn overwrite_with(self, other: Self) -> Self {
                        Self { #overwrite_with_fields }
                    }
                }
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
        /// merging all sources, but each individual source can be missing
        /// required values.
        ///
        /// These types implement `serde::Deserialize`.
        mod raw {
            use super::*;
            use ::confique::serde::{Deserialize, de::IntoDeserializer};

            #contents
        }
    }
}

fn gen_root_mod(input: &Input, visibility: &TokenStream) -> TokenStream {
    let mut out = TokenStream::new();
    visit(input, |node, path| {
        if let Node::Internal { name, doc, children } = node {
            let type_name = to_camel_case(name);

            let user_fields = collect_tokens(children, |node| {
                match node {
                    Node::Leaf { name, doc, ty, .. } => quote! {
                        #( #[doc = #doc] )*
                        #visibility #name: #ty,
                    },
                    Node::Internal { name, .. } => {
                        let child_type_name = to_camel_case(name);
                        quote! {
                            #visibility #name: #child_type_name,
                        }
                    },
                }
            });

            let try_from_fields = collect_tokens(children, |node| {
                match node {
                    Node::Leaf { name, ty, .. } => {
                        if as_option(ty).is_some() {
                            // If this value is optional, we just move it as it can never fail.
                            quote! { #name: src.#name, }
                        } else {
                            // Otherwise, we return an error if the value hasn't been specified.
                            let path = match path.is_empty() {
                                true => name.to_string(),
                                false => format!("{}.{}", path.join("."), name),
                            };

                            quote! {
                                #name: src.#name.ok_or(confique::TryFromError { path: #path })?,
                            }
                        }
                    },
                    Node::Internal { name, .. } => quote! {
                        #name: std::convert::TryFrom::try_from(src.#name)?,
                    },
                }
            });

            out.extend(quote! {
                #( #[doc = #doc] )*
                #[derive(Debug)]
                #visibility struct #type_name {
                    #user_fields
                }

                impl std::convert::TryFrom<raw::#type_name> for #type_name {
                    type Error = ::confique::TryFromError;
                    fn try_from(src: raw::#type_name) -> Result<Self, Self::Error> {
                        Ok(Self {
                            #try_from_fields
                        })
                    }
                }
            });
        }
    });

    out
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
    visit(input, |node, path| {
        match node {
            Node::Internal { doc, .. } => {
                write_doc(&mut out, doc);

                // If a new subsection starts, we always print the header, even if not
                // strictly necessary.
                if path.is_empty() {
                    add_empty_line(&mut out);
                } else {
                    writeln!(out, "[{}]", path.join(".")).unwrap();
                }
            }

            Node::Leaf { doc, name, ty, default, example } => {
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

        if let Node::Internal { children, .. } = node {
            for child in children.iter().rev() {
                let mut child_path = path.clone();
                child_path.push(child.name().to_string());
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

fn to_camel_case(ident: &Ident) -> Ident {
    use heck::CamelCase;

    Ident::new(&ident.to_string().to_camel_case(), ident.span())
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
