
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
