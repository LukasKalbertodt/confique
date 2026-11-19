//! Implementation of the `layer!` macro.

use std::collections::HashSet;

use proc_macro2::{Group, Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream, Parser},
    Error, Token,
};

use crate::{
    gen::{deserializable_expr, deserialize_into_method_name},
    ir::Expr,
    parse::{at_value_end, parse_list, peek_braces, peek_colon},
};


/// An entry of the form `name: value`.
struct Field {
    name: Ident,
    value: Value,
}

enum Value {
    /// `{ field: value, ... }`: a nested configuration.
    Nested(Vec<Field>),

    /// `#(expr)`: assigned to the field as is. This is the `(expr)`.
    Raw(Group),

    /// Everything else: deserialized into the field.
    Deserialize(Expr),
}


// ===== Parsing ==================================================================

fn parse_fields(input: ParseStream) -> Result<Vec<Field>, Error> {
    let fields = parse_list(input, Field::parse)?;

    let mut names = HashSet::new();
    for field in &fields {
        let name = field.name.unraw().to_string();
        if names.contains(&name) {
            return Err(Error::new(field.name.span(), format!("duplicate field `{name}`")));
        }
        names.insert(name);
    }

    Ok(fields)
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let name: Ident = input.parse()?;
        let _: Token![:] = input.parse()?;

        let value = if input.peek(Token![#]) && input.peek2(syn::token::Paren) {
            let _: Token![#] = input.parse()?;
            Value::Raw(input.parse()?)
        } else if is_nested(input) {
            let content;
            syn::braced!(content in input);
            Value::Nested(parse_fields(&content)?)
        } else {
            Value::Deserialize(Expr::parse_with_rust(input)?)
        };

        Ok(Self { name, value })
    }
}

/// Returns `true` if `input` starts with a nested configuration: a `{...}`
/// that is the whole value and starts with `<ident>:`. That distinguishes it
/// from maps, which have literals as keys.
fn is_nested(input: ParseStream) -> bool {
    let fork = input.fork();
    let whole_value = fork.parse::<Group>().is_ok() && at_value_end(&fork);
    whole_value && peek_braces(input, |content| {
        !content.peek(syn::Lit) && content.call(Ident::parse_any).is_ok() && peek_colon(content)
    })
}


// ===== Code generation ==========================================================

pub(crate) fn run(input: TokenStream) -> Result<TokenStream, Error> {
    let parser = |input: ParseStream| {
        let ty: syn::TypePath = input.parse()?;
        let content;
        syn::braced!(content in input);
        Ok((ty, parse_fields(&content)?))
    };
    let (ty, fields) = parser.parse2(input)?;

    // The variable and the label are not visible to the expressions passed by
    // the user, thanks to this span.
    let ctx = Context {
        layer: Ident::new("layer", Span::mixed_site()),
        label: syn::Lifetime::new("'confique_layer", Span::mixed_site()),
    };
    let Context { layer, label } = &ctx;

    let mut assignments = TokenStream::new();
    ctx.gen_fields(&fields, &quote! { #layer }, "", &mut assignments);

    Ok(quote! {
        #label: {
            let mut #layer = <<#ty as confique::Config>::Layer as confique::Layer>::empty();
            #assignments
            std::result::Result::Ok::<_, confique::Error>(#layer)
        }
    })
}

struct Context {
    /// The variable holding the layer.
    layer: Ident,

    /// The label of the outer block, to return errors via `break`. A closure
    /// would be simpler, but then `?` and `.await` in the user's expressions
    /// would not work as expected.
    label: syn::Lifetime,
}

impl Context {
    /// Generates the statements assigning all `fields` of `parent`, which is
    /// an expression evaluating to a layer, like `layer.http`. `path_prefix`
    /// is the path of `parent` followed by a dot, like `http.`, or empty for
    /// the root layer.
    fn gen_fields(
        &self,
        fields: &[Field],
        parent: &TokenStream,
        path_prefix: &str,
        out: &mut TokenStream,
    ) {
        for Field { name, value } in fields {
            let path = format!("{path_prefix}{}", name.unraw());
            match value {
                Value::Nested(fields) => {
                    self.gen_fields(fields, &quote! { #parent.#name }, &format!("{path}."), out);
                }

                // Assigns `Some(expr)` to leaf fields and `expr` to nested
                // ones, see `RawAssignableField`.
                Value::Raw(group) => {
                    let expr = group.stream();
                    let span = Span::mixed_site().located_at(group.span());
                    out.extend(quote_spanned! {span=>
                        confique::internal::RawValueAssigner {
                            field: &mut #parent.#name,
                            value: #expr,
                        }.set();
                    });
                }

                // The layer has a `deserialize_into_{field}` method for every
                // field. The field access before that has no effect, but for
                // an unknown field, it leads to an additional and much better
                // compiler error.
                Value::Deserialize(expr) => {
                    let Context { label, .. } = self;
                    let expr = deserializable_expr(expr);
                    let method = deserialize_into_method_name(name);
                    let err = Ident::new("err", Span::mixed_site());
                    let res = quote! {
                        #parent.#method(confique::internal::into_deserializer(#expr))
                    };
                    out.extend(quote! {
                        let _ = &#parent.#name;
                        if let std::result::Result::Err(#err)
                            = confique::internal::map_layer_field_err(#res, #path)
                        {
                            break #label std::result::Result::Err(#err);
                        }
                    });
                }
            }
        }
    }
}
