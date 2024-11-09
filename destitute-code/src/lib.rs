mod gen_struct;
mod input;

use proc_macro2::TokenStream;

pub fn derive_destitute(item: TokenStream) -> TokenStream {
    let input = match syn::parse2::<input::InputStruct>(item) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };

    gen_struct::generate_destitute_struct(&input)
}
