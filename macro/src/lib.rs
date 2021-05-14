use proc_macro::TokenStream as TokenStream1;


mod gen;
mod ir;


#[proc_macro_derive(Config, attributes(config))]
pub fn config(input: TokenStream1) -> TokenStream1 {
    syn::parse2::<syn::DeriveInput>(input.into())
        .and_then(ir::Input::from_ast)
        .map(gen::gen)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
