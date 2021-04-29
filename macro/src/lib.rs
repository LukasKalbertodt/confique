use proc_macro::TokenStream as TokenStream1;


mod ast;
mod gen;
mod parse;


/// Defines a configuration in a special syntax. TODO: explain what this
/// generates.
#[proc_macro]
pub fn config(input: TokenStream1) -> TokenStream1 {
    syn::parse2::<ast::Input>(input.into())
        .map(gen::gen)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
