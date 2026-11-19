use proc_macro::TokenStream as TokenStream1;


mod gen;
mod ir;
mod layer;
mod parse;
mod scan;
mod util;


#[proc_macro_derive(Config, attributes(config))]
pub fn config(input: TokenStream1) -> TokenStream1 {
    syn::parse2::<syn::DeriveInput>(input.into())
        .and_then(ir::Input::from_ast)
        .map(gen::gen)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro]
pub fn layer(input: TokenStream1) -> TokenStream1 {
    layer::run(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
