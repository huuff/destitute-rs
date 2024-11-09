use proc_macro2::TokenStream;
use quote::quote;

pub fn derive_destitute(item: TokenStream) -> TokenStream {
    let input = syn::parse2::<syn::DeriveInput>(item).unwrap();
    let name = input.ident;
    let vis = input.vis;

    let fields = match input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
            ..
        }) => named,
        _ => unimplemented!("only structs with named fields can derive `Destitute`"),
    };
    let fields = fields.into_iter().map(MaybeDestituteField::from);

    let destitute_name = syn::parse_str::<syn::Ident>(&format!("Destitute{name}")).unwrap();

    quote! {
        #vis struct #destitute_name {
            #(#fields,)*
        }
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
