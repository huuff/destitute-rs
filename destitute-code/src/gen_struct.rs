use crate::input::InputStruct;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub fn generate_destitute_struct(input: &InputStruct) -> TokenStream {
    let fields = input.fields.named.iter().map(|field| {
        if let Some(destitute_attr) = find_destitute_attr(field.attrs.iter()) {
            to_destitute_field(field, destitute_attr).to_token_stream()
        } else {
            field.to_token_stream()
        }
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

fn to_destitute_field(field: &syn::Field, _destitute_attr: &syn::Attribute) -> syn::Field {
    let ty = &field.ty;
    let mut destitute_field = field.clone();
    destitute_field
        .attrs
        .retain(|attr| !attr.path().is_ident("destitute"));
    destitute_field.ty = syn::parse_quote!(Option<#ty>);
    destitute_field
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
