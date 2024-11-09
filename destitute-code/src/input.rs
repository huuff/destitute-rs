use syn::spanned::Spanned;

pub struct InputStruct {
    pub attrs: Vec<syn::Attribute>,
    pub ident: syn::Ident,
    pub vis: syn::Visibility,
    pub fields: syn::FieldsNamed,
}

impl syn::parse::Parse for InputStruct {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input = syn::DeriveInput::parse(input)?;
        let span = input.span();

        let ident = input.ident;
        let attrs = input.attrs;
        let vis = input.vis;
        let fields = match input.data {
            syn::Data::Struct(syn::DataStruct {
                fields: syn::Fields::Named(named),
                ..
            }) => named,
            _ => {
                return Err(syn::Error::new(
                    span,
                    "only struct with named fields can derive `Destitute`",
                ))
            }
        };

        Ok(InputStruct {
            ident,
            vis,
            attrs,
            fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn correctly_parses_input() {
        // ARRANGE
        let input = quote::quote! {
            #[allow(dead_code)]
            struct Example {
                field1: u8,
                field2: u8,
            }
        };

        // ACT
        let input = syn::parse2::<InputStruct>(input).unwrap();

        // ASSERT
        assert_eq!(input.ident.to_string(), "Example");
        assert!(matches!(input.vis, syn::Visibility::Inherited));
        assert_eq!(
            input.fields.named.to_token_stream().to_string(),
            quote::quote! {
                field1: u8,
                field2: u8,
            }
            .to_string()
        );
        let expected_attr: syn::Attribute = syn::parse_quote!(#[allow(dead_code)]);
        assert_eq!(
            input.attrs[0].to_token_stream().to_string(),
            expected_attr.to_token_stream().to_string()
        );
    }

    #[test]
    fn fails_on_tuple_struct() {
        // ARRANGE
        let input = quote::quote!(struct Example(u8, u8));

        // ACT
        let result = syn::parse2::<InputStruct>(input);

        // ASSERT
        assert!(result.is_err());
    }
}
