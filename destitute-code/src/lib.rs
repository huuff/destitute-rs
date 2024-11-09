use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

pub fn derive_destitute(item: TokenStream) -> TokenStream {
    let input = match syn::parse2::<InputStruct>(item) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };

    let vis = &input.vis;
    let fields = input.fields.named.iter().map(MaybeDestituteField::from);
    let destitute_name = quote::format_ident!("Destitute{}", input.ident);

    quote! {
        #vis struct #destitute_name {
            #(#fields,)*
        }
    }
}

struct InputStruct {
    ident: syn::Ident,
    vis: syn::Visibility,
    fields: syn::FieldsNamed,
}

impl syn::parse::Parse for InputStruct {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input = syn::DeriveInput::parse(input)?;
        let span = input.span();

        let ident = input.ident;
        let vis = input.vis;
        let fields = match input.data {
            syn::Data::Struct(syn::DataStruct {
                fields: syn::Fields::Named(named),
                ..
            }) => named,
            // _ => unimplemented!("only structs with named fields can derive `Destitute`"),
            _ => {
                return Err(syn::Error::new(
                    span,
                    "only struct with named fields can derive `Destitute`",
                ))
            }
        };

        Ok(InputStruct { ident, vis, fields })
    }
}

struct MaybeDestituteField<'a>(&'a syn::Field);

impl<'a> From<&'a syn::Field> for MaybeDestituteField<'a> {
    fn from(field: &'a syn::Field) -> Self {
        Self(field)
    }
}

impl<'a> quote::ToTokens for MaybeDestituteField<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let destitute_attr = self
            .0
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("destitute"));

        if let Some(_destitute_attr) = destitute_attr {
            let vis = &self.0.vis;
            let name = &self.0.ident;
            let ty = &self.0.ty;
            // TODO: gotta remove the destitute attr to not recursively add it
            // let attrs = &self.0.attrs;
            // quote!(#(#attrs)* #vis #name: Option<#ty>).to_tokens(tokens);
            quote!( #vis #name: Option<#ty>).to_tokens(tokens);
        } else {
            let field = &self.0;
            quote!(#field).to_tokens(tokens)
        }
    }
}
