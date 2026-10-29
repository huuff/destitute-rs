//! Implementation of `#[derive(Destitute)]`. Use the `destitute` crate instead.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, Ident, LitStr, Meta, Result, Token,
    parse_macro_input, punctuated::Punctuated, spanned::Spanned,
};

#[proc_macro_derive(Destitute, attributes(destitute))]
pub fn derive_destitute(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[derive(Default)]
struct StructOpts {
    name: Option<Ident>,
    derives: Vec<Meta>,
    attrs: Vec<Meta>,
    no_enum: bool,
    serde: bool,
    enum_name: Option<Ident>,
    enum_derives: Vec<Meta>,
    enum_attrs: Vec<Meta>,
}

fn parse_struct_opts(attrs: &[Attribute]) -> Result<StructOpts> {
    let mut opts = StructOpts::default();
    for attr in attrs.iter().filter(|a| a.path().is_ident("destitute")) {
        for meta in attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
            let key = meta
                .path()
                .get_ident()
                .map(Ident::to_string)
                .unwrap_or_default();
            match (key.as_str(), &meta) {
                ("name", Meta::NameValue(nv)) => opts.name = Some(ident_from(&nv.value)?),
                ("enum_name", Meta::NameValue(nv)) => {
                    opts.enum_name = Some(ident_from(&nv.value)?);
                }
                ("no_enum", Meta::Path(_)) => opts.no_enum = true,
                #[cfg(feature = "serde")]
                ("serde", Meta::Path(_)) => opts.serde = true,
                #[cfg(not(feature = "serde"))]
                ("serde", Meta::Path(_)) => {
                    return Err(Error::new(
                        meta.span(),
                        "`#[destitute(serde)]` needs the `serde` feature of the `destitute` crate",
                    ));
                }
                ("derive", Meta::List(list)) => opts.derives.extend(list_items(list)?),
                ("enum_derive", Meta::List(list)) => opts.enum_derives.extend(list_items(list)?),
                ("attr", Meta::List(list)) => opts.attrs.extend(list_items(list)?),
                ("enum_attr", Meta::List(list)) => opts.enum_attrs.extend(list_items(list)?),
                _ => {
                    return Err(Error::new(
                        meta.span(),
                        "unknown `#[destitute(...)]` option; expected one of \
                         `name = \"..\"`, `derive(..)`, `attr(..)`, `no_enum`, `serde`, \
                         `enum_name = \"..\"`, `enum_derive(..)`, `enum_attr(..)`",
                    ));
                }
            }
        }
    }
    Ok(opts)
}

fn list_items(list: &syn::MetaList) -> Result<Vec<Meta>> {
    Ok(list
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
        .into_iter()
        .collect())
}

fn ident_from(expr: &syn::Expr) -> Result<Ident> {
    let lit: LitStr = syn::parse2(quote!(#expr))
        .map_err(|_| Error::new(expr.span(), "expected a string literal"))?;
    lit.parse()
}

fn is_mandatory(field: &Field) -> Result<bool> {
    let mut mandatory = false;
    for attr in field
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("destitute"))
    {
        for meta in attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
            if meta.path().is_ident("mandatory") {
                mandatory = true;
            } else {
                return Err(Error::new(
                    meta.span(),
                    "unknown field option; only `#[destitute(mandatory)]` is supported",
                ));
            }
        }
    }
    Ok(mandatory)
}

/// `Option<T>`, `core::option::Option<T>` or `std::option::Option<T>`, by spelling.
fn is_option(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() {
        return false;
    }
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let segments: Vec<&str> = segments.iter().map(String::as_str).collect();
    let prefix_ok = matches!(
        segments.as_slice(),
        ["Option"] | ["core" | "std", "option", "Option"]
    );
    prefix_ok
        && matches!(
            &path.path.segments.last().unwrap().arguments,
            syn::PathArguments::AngleBracketed(args) if args.args.len() == 1
        )
}

fn doc_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|a| a.path().is_ident("doc"))
}

fn expand(input: DeriveInput) -> Result<TokenStream> {
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(Error::new(
                    input.ident.span(),
                    "`Destitute` requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new(
                input.ident.span(),
                "`Destitute` can only be derived for structs",
            ));
        }
    };

    let opts = parse_struct_opts(&input.attrs)?;
    let vis = &input.vis;
    let name = &input.ident;
    let name_str = name.to_string();
    let dest = opts
        .name
        .unwrap_or_else(|| format_ident!("Destitute{}", name));
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let field_default = if opts.serde {
        quote!(#[serde(default)])
    } else {
        quote!()
    };

    let mut dest_fields = Vec::new();
    let mut wrap = Vec::new();
    let mut unwrap = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().expect("named field");
        let ty = &field.ty;
        let fvis = &field.vis;
        let docs = doc_attrs(&field.attrs);
        if is_mandatory(field)? {
            dest_fields.push(quote!(#(#docs)* #fvis #ident: #ty));
            wrap.push(quote!(#ident: self.#ident));
            unwrap.push(quote!(#ident: self.#ident));
        } else if is_option(ty) {
            dest_fields.push(quote!(#(#docs)* #field_default #fvis #ident: #ty));
            wrap.push(quote!(#ident: self.#ident));
            unwrap.push(quote!(#ident: self.#ident));
        } else {
            let field_str = ident.to_string();
            dest_fields
                .push(quote!(#(#docs)* #field_default #fvis #ident: ::core::option::Option<#ty>));
            wrap.push(quote!(#ident: ::core::option::Option::Some(self.#ident)));
            unwrap.push(quote! {
                #ident: match self.#ident {
                    ::core::option::Option::Some(v) => v,
                    ::core::option::Option::None => {
                        return ::core::result::Result::Err(::destitute::MissingField {
                            type_name: #name_str,
                            field: #field_str,
                        });
                    }
                }
            });
        }
    }

    let derives = &opts.derives;
    let attrs = &opts.attrs;
    let (serde_derive, serde_struct_attr, serde_enum_attr) = if opts.serde {
        (
            quote!(#[derive(::destitute::__private::serde::Serialize, ::destitute::__private::serde::Deserialize)]),
            quote!(#[serde(crate = "::destitute::__private::serde")]),
            quote!(#[serde(crate = "::destitute::__private::serde", untagged)]),
        )
    } else {
        (quote!(), quote!(), quote!())
    };
    let dest_doc = format!("All-optional form of [`{name_str}`].");
    let mut out = quote! {
        #[doc = #dest_doc]
        #[derive(#(#derives),*)]
        #serde_derive
        #serde_struct_attr
        #(#[#attrs])*
        #vis struct #dest #ty_generics #where_clause {
            #(#dest_fields,)*
        }

        impl #impl_generics #name #ty_generics #where_clause {
            /// Wrap every non-mandatory field in `Some`.
            #vis fn into_destitute(self) -> #dest #ty_generics {
                #dest { #(#wrap,)* }
            }
        }

        impl #impl_generics #dest #ty_generics #where_clause {
            /// Promote into the complete form, failing on the first `None` field.
            #vis fn try_undestitute(self) -> ::core::result::Result<#name #ty_generics, ::destitute::MissingField> {
                ::core::result::Result::Ok(#name { #(#unwrap,)* })
            }
        }

        impl #impl_generics ::core::convert::From<#name #ty_generics> for #dest #ty_generics #where_clause {
            fn from(value: #name #ty_generics) -> Self {
                value.into_destitute()
            }
        }

        impl #impl_generics ::core::convert::TryFrom<#dest #ty_generics> for #name #ty_generics #where_clause {
            type Error = ::destitute::MissingField;
            fn try_from(value: #dest #ty_generics) -> ::core::result::Result<Self, Self::Error> {
                value.try_undestitute()
            }
        }
    };

    if !opts.no_enum {
        let en = opts
            .enum_name
            .unwrap_or_else(|| format_ident!("MaybeDestitute{}", name));
        let enum_derives = &opts.enum_derives;
        let enum_attrs = &opts.enum_attrs;
        let enum_doc = format!("Either a complete [`{name_str}`] or a [`{dest}`].");
        out.extend(quote! {
            #[doc = #enum_doc]
            #[derive(#(#enum_derives),*)]
            #serde_derive
            #serde_enum_attr
            #(#[#enum_attrs])*
            #vis enum #en #ty_generics #where_clause {
                /// Every field is present. Tried first when deserializing.
                Complete(#name #ty_generics),
                /// Some fields may be missing.
                Destitute(#dest #ty_generics),
            }

            impl #impl_generics #en #ty_generics #where_clause {
                /// Flatten into the all-optional form.
                #vis fn into_destitute(self) -> #dest #ty_generics {
                    match self {
                        Self::Complete(v) => v.into_destitute(),
                        Self::Destitute(v) => v,
                    }
                }

                /// Promote into the complete form, failing on the first `None` field.
                #vis fn try_undestitute(self) -> ::core::result::Result<#name #ty_generics, ::destitute::MissingField> {
                    match self {
                        Self::Complete(v) => ::core::result::Result::Ok(v),
                        Self::Destitute(v) => v.try_undestitute(),
                    }
                }
            }

            impl #impl_generics ::core::convert::From<#name #ty_generics> for #en #ty_generics #where_clause {
                fn from(value: #name #ty_generics) -> Self {
                    Self::Complete(value)
                }
            }

            impl #impl_generics ::core::convert::From<#dest #ty_generics> for #en #ty_generics #where_clause {
                fn from(value: #dest #ty_generics) -> Self {
                    Self::Destitute(value)
                }
            }
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(src: &str) -> String {
        expand(syn::parse_str(src).unwrap())
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn rejects_enums_and_tuple_structs() {
        assert_eq!(
            err("enum E { A }"),
            "`Destitute` can only be derived for structs"
        );
        assert_eq!(
            err("struct T(u8);"),
            "`Destitute` requires a struct with named fields"
        );
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(
            err("#[destitute(bogus)] struct S { a: u8 }")
                .starts_with("unknown `#[destitute(...)]`")
        );
        assert!(
            err("struct S { #[destitute(optional)] a: u8 }").starts_with("unknown field option")
        );
    }

    #[test]
    fn option_fields_stay_flat() {
        for ty in [
            "Option<u8>",
            "::core::option::Option<u8>",
            "std::option::Option<u8>",
        ] {
            assert!(is_option(&syn::parse_str(ty).unwrap()), "{ty}");
        }
        for ty in ["Vec<u8>", "Option", "foo::Option<u8>", "Option<u8, u8>"] {
            assert!(!is_option(&syn::parse_str(ty).unwrap()), "{ty}");
        }
    }

    #[test]
    fn expands_without_error() {
        let src = r#"
            #[destitute(name = "Partial", derive(Debug), no_enum)]
            pub struct S<T: Clone> where T: Default { #[destitute(mandatory)] a: T, b: u8 }
        "#;
        let out = expand(syn::parse_str(src).unwrap()).unwrap().to_string();
        assert!(out.contains("pub struct Partial"));
        assert!(!out.contains("enum"));
    }
}
