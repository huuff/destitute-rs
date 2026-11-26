//! Implementation of `#[derive(Destitute)]`. Use the `destitute` crate instead.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, Ident, LitStr, Meta, Result, Token,
    ext::IdentExt, parse_macro_input, punctuated::Punctuated, spanned::Spanned,
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
    /// Span of the `accessors` option, when given.
    accessors: Option<proc_macro2::Span>,
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
                ("accessors", Meta::Path(_)) => opts.accessors = Some(meta.span()),
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
                         `name = \"..\"`, `derive(..)`, `attr(..)`, `no_enum`, `accessors`, \
                         `serde`, `enum_name = \"..\"`, `enum_derive(..)`, `enum_attr(..)`",
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

#[derive(Default)]
struct FieldOpts {
    mandatory: bool,
    /// Attributes to put verbatim on the generated field.
    attrs: Vec<Meta>,
}

fn parse_field_opts(field: &Field) -> Result<FieldOpts> {
    let mut opts = FieldOpts::default();
    for attr in field
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("destitute"))
    {
        for meta in attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
            let key = meta
                .path()
                .get_ident()
                .map(Ident::to_string)
                .unwrap_or_default();
            match (key.as_str(), &meta) {
                ("mandatory", Meta::Path(_)) => opts.mandatory = true,
                ("attr", Meta::List(list)) => opts.attrs.extend(list_items(list)?),
                _ => {
                    return Err(Error::new(
                        meta.span(),
                        "unknown field option; expected `mandatory` or `attr(..)`",
                    ));
                }
            }
        }
    }
    Ok(opts)
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
    if let (Some(span), true) = (opts.accessors, opts.no_enum) {
        return Err(Error::new(
            span,
            "`accessors` generates methods on the enum, which `no_enum` removes",
        ));
    }
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
    let mut accessors = Vec::new();
    let mut enum_methods = vec!["into_destitute".to_owned(), "try_undestitute".to_owned()];
    for field in fields {
        let ident = field.ident.as_ref().expect("named field");
        let ty = &field.ty;
        let fvis = &field.vis;
        let docs = doc_attrs(&field.attrs);
        let field_opts = parse_field_opts(field)?;
        let fattrs = &field_opts.attrs;
        if field_opts.mandatory {
            dest_fields.push(quote!(#(#docs)* #(#[#fattrs])* #fvis #ident: #ty));
            wrap.push(quote!(#ident: self.#ident));
            unwrap.push(quote!(#ident: self.#ident));
            if opts.accessors.is_some() {
                let ident_mut = format_ident!("{}_mut", ident);
                for method in [ident, &ident_mut] {
                    let method = method.unraw().to_string();
                    if enum_methods.contains(&method) {
                        return Err(Error::new(
                            ident.span(),
                            format!(
                                "`accessors` would generate two methods named `{method}` on the enum"
                            ),
                        ));
                    }
                    enum_methods.push(method);
                }
                let doc = format!("The `{}` field, whichever variant this is.", ident.unraw());
                accessors.push(quote! {
                    #[doc = #doc]
                    #[allow(dead_code)]
                    #fvis fn #ident(&self) -> &#ty {
                        match self {
                            Self::Complete(v) => &v.#ident,
                            Self::Destitute(v) => &v.#ident,
                        }
                    }

                    #[doc = #doc]
                    #[allow(dead_code)]
                    #fvis fn #ident_mut(&mut self) -> &mut #ty {
                        match self {
                            Self::Complete(v) => &mut v.#ident,
                            Self::Destitute(v) => &mut v.#ident,
                        }
                    }
                });
            }
        } else if is_option(ty) {
            dest_fields.push(quote!(#(#docs)* #field_default #(#[#fattrs])* #fvis #ident: #ty));
            wrap.push(quote!(#ident: self.#ident));
            unwrap.push(quote!(#ident: self.#ident));
        } else {
            let field_str = ident.to_string();
            dest_fields.push(quote! {
                #(#docs)* #field_default #(#[#fattrs])*
                #fvis #ident: ::core::option::Option<#ty>
            });
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

                #(#accessors)*
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
    use quote::ToTokens;

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

    /// Attributes on each field of the generated struct `name`, as `(field, [meta, ..])`.
    fn field_attrs(src: &str, name: &str) -> Vec<(String, Vec<String>)> {
        let out = expand(syn::parse_str(src).unwrap()).unwrap();
        let file: syn::File = syn::parse2(out).unwrap();
        let item = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(s) if s.ident == name => Some(s),
                _ => None,
            })
            .unwrap();
        item.fields
            .iter()
            .map(|f| {
                let attrs = f
                    .attrs
                    .iter()
                    .filter(|a| !a.path().is_ident("doc"))
                    .map(|a| a.meta.to_token_stream().to_string())
                    .collect();
                (f.ident.as_ref().unwrap().to_string(), attrs)
            })
            .collect()
    }

    fn meta(tokens: TokenStream) -> String {
        tokens.to_string()
    }

    #[test]
    fn field_attr_is_passed_through_on_every_kind_of_field() {
        let src = r#"
            struct S {
                #[destitute(mandatory, attr(builder(default)))]
                #[builder(dropped)]
                a: u8,
                #[destitute(attr(builder(into), serde(rename = "bee")))]
                b: u8,
                #[destitute(attr(builder(name = sea)))]
                #[destitute(attr(allow(dead_code)))]
                c: Option<u8>,
                d: u8,
            }
        "#;
        assert_eq!(
            field_attrs(src, "DestituteS"),
            [
                ("a".to_owned(), vec![meta(quote!(builder(default)))]),
                (
                    "b".to_owned(),
                    vec![
                        meta(quote!(builder(into))),
                        meta(quote!(serde(rename = "bee")))
                    ]
                ),
                (
                    "c".to_owned(),
                    vec![
                        meta(quote!(builder(name = sea))),
                        meta(quote!(allow(dead_code)))
                    ]
                ),
                ("d".to_owned(), vec![]),
            ]
        );
    }

    #[test]
    fn rejects_malformed_field_options() {
        for src in [
            "struct S { #[destitute(mandatory(x))] a: u8 }",
            "struct S { #[destitute(mandatory = true)] a: u8 }",
            "struct S { #[destitute(attr)] a: u8 }",
            "struct S { #[destitute(attr = \"x\")] a: u8 }",
        ] {
            assert!(err(src).starts_with("unknown field option"), "{src}");
        }
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

    /// Signatures of the inherent methods generated on the type `name`.
    fn methods(src: &str, name: &str) -> Vec<String> {
        let out = expand(syn::parse_str(src).unwrap()).unwrap();
        let file: syn::File = syn::parse2(out).unwrap();
        file.items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Impl(i) if i.trait_.is_none() => Some(i),
                _ => None,
            })
            .filter(
                |i| matches!(&*i.self_ty, syn::Type::Path(p) if p.path.segments[0].ident == name),
            )
            .flat_map(|i| &i.items)
            .filter_map(|item| match item {
                syn::ImplItem::Fn(f) => {
                    let (vis, sig) = (&f.vis, &f.sig);
                    Some(quote!(#vis #sig).to_string())
                }
                _ => None,
            })
            .collect()
    }

    const ACCESSORS_SRC: &str = r#"
        pub struct S<T> {
            #[destitute(mandatory)] pub a: T,
            #[destitute(mandatory)] pub(crate) b: u8,
            #[destitute(mandatory)] c: u8,
            pub d: u8,
            pub e: Option<u8>,
        }
    "#;

    #[test]
    fn accessors_are_opt_in() {
        assert_eq!(methods(ACCESSORS_SRC, "MaybeDestituteS").len(), 2);
    }

    #[test]
    fn accessors_cover_mandatory_fields_with_the_field_visibility() {
        let src = format!("#[destitute(accessors)] {ACCESSORS_SRC}");
        assert_eq!(
            methods(&src, "MaybeDestituteS")[2..],
            [
                quote!(pub fn a(&self) -> &T),
                quote!(pub fn a_mut(&mut self) -> &mut T),
                quote!(pub(crate) fn b(&self) -> &u8),
                quote!(pub(crate) fn b_mut(&mut self) -> &mut u8),
                quote!(fn c(&self) -> &u8),
                quote!(fn c_mut(&mut self) -> &mut u8),
            ]
            .map(|t| t.to_string())
        );
    }

    #[test]
    fn rejects_accessors_without_the_enum() {
        assert_eq!(
            err("#[destitute(accessors, no_enum)] struct S { #[destitute(mandatory)] a: u8 }"),
            "`accessors` generates methods on the enum, which `no_enum` removes"
        );
    }

    #[test]
    fn rejects_accessor_name_clashes() {
        assert_eq!(
            err("#[destitute(accessors)] struct S { #[destitute(mandatory)] into_destitute: u8 }"),
            "`accessors` would generate two methods named `into_destitute` on the enum"
        );
        assert_eq!(
            err("#[destitute(accessors)] struct S { \
                 #[destitute(mandatory)] a: u8, #[destitute(mandatory)] a_mut: u8 }"),
            "`accessors` would generate two methods named `a_mut` on the enum"
        );
        // A non-mandatory field gets no accessor, so it cannot clash.
        assert!(
            expand(
                syn::parse_str("#[destitute(accessors)] struct S { try_undestitute: u8 }").unwrap()
            )
            .is_ok()
        );
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
