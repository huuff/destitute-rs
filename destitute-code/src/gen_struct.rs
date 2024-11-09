use crate::input::InputStruct;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub fn generate_destitute_struct(input: &InputStruct) -> TokenStream {
    let fields = input.fields.named.iter().map(|field| {
        find_destitute_attr(field.attrs.iter())
            .map(|_| {
                let ty = &field.ty;
                let mut optional_field = field.clone();
                optional_field
                    .attrs
                    .retain(|attr| !attr.path().is_ident("destitute"));
                optional_field.ty = syn::parse_quote!(Option<#ty>);
                optional_field.into_token_stream()
            })
            .unwrap_or(field.to_token_stream())
    });

    let destitute_name = quote::format_ident!("Destitute{}", input.ident);
    let vis = &input.vis;
    let attrs = &input.attrs;

    // TODO fully qualify option?
    // TODO do not nest options
    quote! {
        #(#attrs)*
        #vis struct #destitute_name {
            #(#fields,)*
        }
    }
}

fn find_destitute_attr<'a>(
    mut iter: impl Iterator<Item = &'a syn::Attribute>,
) -> Option<&'a syn::Attribute> {
    iter.find(|attr| attr.path().is_ident("destitute"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_generates_struct() {
        // ARRANGE
        let input = InputStruct {
            attrs: vec![syn::parse_quote!(#[allow(dead_code)])],
            ident: syn::Ident::new("Example", proc_macro2::Span::call_site()),
            vis: syn::Visibility::Public(Default::default()),
            fields: syn::parse_quote!({
                #[destitute]
                field1: u8,
                field2: u8
            }),
        };

        let expected = quote! {
            #[allow(dead_code)]
            pub struct DestituteExample {
                field1: Option<u8>,
                field2: u8,
            }
        };

        // ACT
        let destitute_struct = generate_destitute_struct(&input);

        // ASSERT
        assert_eq!(destitute_struct.to_string(), expected.to_string());
    }
}
