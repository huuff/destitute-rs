use proc_macro::TokenStream;

#[proc_macro_derive(Destitute, attributes(destitute))]
pub fn derive_destitute(item: TokenStream) -> TokenStream {
    destitute_code::derive_destitute(item.into()).into()
}
