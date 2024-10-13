use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident};

#[proc_macro_derive(Destitute, attributes(destitute))]
pub fn derive_destitute(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = input.ident;
    let vis = input.vis;

    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => unimplemented!("only structs with named fields can derive `Destitute`"),
    };
    let fields = fields.into_iter().map(MaybeDestituteField::from);

    let destitute_name = syn::parse_str::<Ident>(&format!("Destitute{name}")).unwrap();

    quote! {
        #vis struct #destitute_name {
            #(#fields,)*
        }
    }
    .into()
}

struct MaybeDestituteField<'a>(&'a Field);

impl<'a> From<&'a Field> for MaybeDestituteField<'a> {
    fn from(field: &'a Field) -> Self {
        Self(field)
    }
}

impl<'a> ToTokens for MaybeDestituteField<'a> {
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
