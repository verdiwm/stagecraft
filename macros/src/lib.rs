use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Field, Fields, FieldsNamed, Ident, ItemEnum, Type};

struct MacroArgs {
    actor: Ident,
}

impl syn::parse::Parse for MacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MacroArgs {
            actor: input.parse()?,
        })
    }
}

/// Returns `None` for `#[call]` or `#[call()]` (→ return `()`),
/// or `Some(ty)` for `#[call(SomeType)]`.
fn parse_call_attr(attr: &syn::Attribute) -> syn::Result<Option<Type>> {
    match &attr.meta {
        syn::Meta::Path(_) => Ok(None),
        syn::Meta::List(list) if list.tokens.is_empty() => Ok(None),
        syn::Meta::List(list) => syn::parse2::<Type>(list.tokens.clone()).map(Some),
        syn::Meta::NameValue(nv) => Err(syn::Error::new_spanned(
            nv,
            "#[call] does not support key=value syntax; use #[call] or #[call(ReturnType)]",
        )),
    }
}

#[proc_macro_attribute]
pub fn message(attr: TokenStream, item: TokenStream) -> TokenStream {
    message_impl(attr, item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn message_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream2> {
    let MacroArgs { actor } = syn::parse(attr)?;
    let mut enum_def: ItemEnum = syn::parse(item)?;

    let enum_vis = enum_def.vis.clone();
    let enum_name = enum_def.ident.clone();
    let trait_name = format_ident!("{}Ext", actor);

    let mut trait_methods: Vec<TokenStream2> = Vec::new();
    let mut impl_methods: Vec<TokenStream2> = Vec::new();

    for variant in &mut enum_def.variants {
        // Find and remove #[call] attr
        let call_pos = variant.attrs.iter().position(|a| a.path().is_ident("call"));
        let ret_ty: Option<Type> = call_pos
            .map(|i| {
                let attr = variant.attrs.remove(i);
                parse_call_attr(&attr).map(|t| t.unwrap_or_else(|| syn::parse_quote!(())))
            })
            .transpose()?;

        let variant_name = &variant.ident;
        let method_name = format_ident!("{}", variant_name.to_string().to_snake_case());

        // Collect param names and types; generate synthetic names for tuple fields.
        let is_tuple = matches!(variant.fields, Fields::Unnamed(_));
        let (param_idents, param_types): (Vec<Ident>, Vec<Type>) = match &variant.fields {
            Fields::Named(f) => f
                .named
                .iter()
                .map(|field| (field.ident.clone().unwrap(), field.ty.clone()))
                .unzip(),
            Fields::Unnamed(f) => f
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, field)| (format_ident!("arg{}", i), field.ty.clone()))
                .unzip(),
            Fields::Unit => (vec![], vec![]),
        };

        let ret = ret_ty.as_ref().map_or_else(|| quote!(()), |t| quote!(#t));
        let sig = quote! {
            async fn #method_name(&self #(, #param_idents: #param_types)*)
                -> ::core::result::Result<#ret, ::stagecraft::ActorDead<()>>
        };

        let body: TokenStream2 = if let Some(ref ret_ty) = ret_ty {
            if is_tuple {
                // Append respond_to as the last tuple element.
                let respond_to_field = Field {
                    attrs: vec![],
                    vis: syn::Visibility::Inherited,
                    mutability: syn::FieldMutability::None,
                    ident: None,
                    colon_token: None,
                    ty: syn::parse_quote! { ::tokio::sync::oneshot::Sender<#ret_ty> },
                };
                match &mut variant.fields {
                    Fields::Unnamed(f) => f.unnamed.push(respond_to_field),
                    _ => unreachable!(),
                }
                quote! {
                    self.call(|tx| #enum_name::#variant_name(#(#param_idents,)* tx)).await
                }
            } else {
                let respond_to: Field = syn::parse_quote! {
                    respond_to: ::tokio::sync::oneshot::Sender<#ret_ty>
                };
                match &mut variant.fields {
                    Fields::Named(f) => f.named.push(respond_to),
                    Fields::Unit => {
                        variant.fields = Fields::Named(FieldsNamed {
                            brace_token: Default::default(),
                            named: std::iter::once(respond_to).collect(),
                        });
                    }
                    Fields::Unnamed(_) => unreachable!(),
                }
                quote! {
                    self.call(|tx| #enum_name::#variant_name { #(#param_idents,)* respond_to: tx }).await
                }
            }
        } else {
            let construction = if is_tuple {
                quote! { #enum_name::#variant_name(#(#param_idents),*) }
            } else if param_idents.is_empty() {
                quote! { #enum_name::#variant_name }
            } else {
                quote! { #enum_name::#variant_name { #(#param_idents),* } }
            };
            quote! {
                self.cast(#construction).await.map_err(|_| ::stagecraft::ActorDead(()))
            }
        };

        trait_methods.push(quote! { #sig; });
        impl_methods.push(quote! { #sig { #body } });
    }

    Ok(quote! {
        #enum_def

        #enum_vis trait #trait_name {
            #(#trait_methods)*
        }

        impl #trait_name for ::stagecraft::Handle<#actor> {
            #(#impl_methods)*
        }
    })
}
