use proc_macro::TokenStream as TokenStream1;


mod gen;
mod ir;
mod parse;
mod util;


#[proc_macro_derive(Config, attributes(config))]
pub fn config(input: TokenStream1) -> TokenStream1 {
    let input = match syn::parse2::<syn::DeriveInput>(input.into()) {
        Err(e) => return e.to_compile_error().into(),
        Ok(i) => i,
    };

    ir::Input::from_ast(input)
        .map(gen::gen)
        .unwrap_or_else(|e| e.write_errors())
        .into()
}
