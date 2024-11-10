use crate::input::InputStruct;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

pub fn generate_destitute_struct(input: &InputStruct) -> TokenStream {
    let fields = input.fields.named.iter().map(|field| {
        if let Some(field_destitution) = FieldDestitution::find_in(field) {
            match to_destitute_field(field, &field_destitution) {
                Ok(field) => field.to_token_stream(),
                Err(err) => err.to_compile_error().into_token_stream(),
            }
        } else {
            field.to_token_stream()
        }
    });

    let destitute_name = quote::format_ident!("Destitute{}", input.ident);
    let vis = &input.vis;
    let attrs = &input.attrs;

    // TODO fully qualify option?
    quote! {
        #(#attrs)*
        #vis struct #destitute_name {
            #(#fields,)*
        }
    }
}

fn to_destitute_field(
    field: &syn::Field,
    _destitution: &FieldDestitution,
) -> syn::Result<syn::Field> {
    let ty = match &field.ty {
        syn::Type::Path(ref ty) => ty,
        _ => {
            return Err(syn::Error::new(
                field.ty.span(),
                "#[destitute] only works on path types like `String` or `Vec<String>`",
            ))
        }
    };

    let mut destitute_field = field.clone();

    // remove the destitute attribute in the output
    destitute_field
        .attrs
        .retain(|attr| !attr.path().is_ident("destitute"));

    // make it optional
    destitute_field.ty = if ty
        .path
        .segments
        .iter()
        .next()
        .is_some_and(|it| it.ident == "Option")
    {
        syn::Type::Path(ty.clone())
    } else {
        syn::parse_quote!(Option<#ty>)
    };

    Ok(destitute_field)
}

/// Parsed configuration inside a `#[destitute]` attribute for a field
#[derive(Default)]
struct FieldDestitution {}

impl FieldDestitution {
    fn find_in(field: &syn::Field) -> Option<Self> {
        field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("destitute"))
            .map(|_| FieldDestitution {})
    }
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

    #[test]
    fn does_not_nest_options() {
        // ARRANGE
        let destitution = FieldDestitution::default();
        let field: syn::Field = syn::parse_quote!(pub field1: Option<u8>);

        // ACT
        let destitute_field = to_destitute_field(&field, &destitution).unwrap();

        // ASSERT
        assert_eq!(
            field.to_token_stream().to_string(),
            destitute_field.to_token_stream().to_string()
        );
    }
}
