
/// Checks if the given type is an `Option` and if so, return the inner type.
///
/// Note: this function clearly shows one of the major shortcomings of proc
/// macros right now: we do not have access to the compiler's type tables and
/// can only check if it "looks" like an `Option`. Of course, stuff can go
/// wrong. But that's the best we can do and it's highly unlikely that someone
/// shadows `Option`.
pub(crate) fn unwrap_option(ty: &syn::Type) -> Option<&syn::Type> {
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

/// Returns `true` if the given type is `Option<_>`.
pub(crate) fn is_option(ty: &syn::Type) -> bool {
    unwrap_option(ty).is_some()
}

/// Tries to extract the type of the item of a field with an array default
/// value. Examples: `&[u32]` -> `u32`, `Vec<String>` -> `String`.
pub(crate) fn get_array_item_type(ty: &syn::Type) -> Option<&syn::Type> {
    match ty {
        // The easy types.
        syn::Type::Slice(slice) => Some(&slice.elem),
        syn::Type::Array(array) => Some(&*array.elem),

        // This is the least clear case. We certainly want to cover `Vec<T>` but
        // ideally some more cases. On the other hand, we just can't really
        // know, so some incorrect guesses are definitely expected here. Most
        // are likely filtered out by applying `gen_meta_default` to it, but
        // some will result in a wrong default value type. But people can
        // always just add a prefix to the literal in those cases.
        //
        // We simply check if the last element in the path has exactly one
        // generic type argument, in which case we use that.
        syn::Type::Path(p) => {
            let args = match &p.path.segments.last().expect("empty type path").arguments {
                syn::PathArguments::AngleBracketed(args) => &args.args,
                _ => return None,
            };

            if args.len() != 1 {
                return None;
            }

            match &args[0] {
                syn::GenericArgument::Type(t) => Some(t),
                _ => None,
            }
        },

        // Just recurse on inner type.
        syn::Type::Reference(r) => get_array_item_type(&r.elem),
        syn::Type::Group(g) => get_array_item_type(&g.elem),
        syn::Type::Paren(p) => get_array_item_type(&p.elem),

        _ => None,
    }
}

/// Tries to extract the key and value types from a map value. Examples:
/// `HashMap<String, u32>` -> `(String, u32)`.
pub(crate) fn get_map_entry_types(ty: &syn::Type) -> Option<(&syn::Type, &syn::Type)> {
    match ty {
        // We simply check if the last element in the path has exactly two
        // generic type arguments, in which case we use those. Otherwise we
        // can't really know.
        syn::Type::Path(p) => {
            let args = match &p.path.segments.last().expect("empty type path").arguments {
                syn::PathArguments::AngleBracketed(args) => &args.args,
                _ => return None,
            };

            if args.len() != 2 {
                return None;
            }

            match (&args[0], &args[1]) {
                (syn::GenericArgument::Type(k), syn::GenericArgument::Type(v)) => Some((k, v)),
                _ => None,
            }
        },

        // Just recurse on inner type.
        syn::Type::Group(g) => get_map_entry_types(&g.elem),
        syn::Type::Paren(p) => get_map_entry_types(&p.elem),

        _ => None,
    }
}
