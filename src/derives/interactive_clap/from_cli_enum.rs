extern crate proc_macro;

use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;


pub fn fn_from_cli(ast: &syn::DeriveInput, variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let cli_name = syn::Ident::new(&format!("Cli{}", name), Span::call_site());

    let mut context_dir = quote! ();

    let mut is_fn_from_default = false;
    
    for attr in ast.attrs.clone() {
        if attr.path.is_ident("interactive_clap".into()) {
            for attr_token in attr.tokens.clone() {
                match attr_token {
                    proc_macro2::TokenTree::Group(group) => {
                        if group.stream().to_string().contains("context") {
                            let group_stream = &group.stream()
                            .into_iter()
                            .enumerate()
                            .filter(|&(i,_)| i != 0 || i != 1)
                            .map(|(_, v)| v)
                            .collect::<Vec<_>>()[2..];
                            context_dir = quote! {#(#group_stream)*};
                        };
                        if group.stream().to_string().contains("fn_from") && group.stream().to_string().contains("default") {
                            is_fn_from_default = true;
                        };
                    }
                    _ => () //abort_call_site!("Only option `TokenTree::Group` is needed")
                }
            }
        };
    };

    if is_fn_from_default {
        // let from_cli_enum_variants = variants.iter().map(|variant| {
        //     let ident = &variant.ident;
        //     let interactive_clap_context_scope_for_cli_name = &syn::Ident::new(format!("InteractiveClapContextScopeFor{}", &name.to_string()).as_str(), Span::call_site());
        //     let select_server_context = &syn::Ident::new(format!("{}Context", &name.to_string()).as_str(), Span::call_site());

        //     match &variant.fields {
        //         syn::Fields::Unnamed(fields) => {
        //             let ty = &fields.unnamed[0].ty;
        //             // quote! { #ident(<#ty as ToCli>::CliVariant) }
        //             quote! {
        //                 #cli_name::#ident(cli_args) => {
        //                     let new_context_scope = #interactive_clap_context_scope_for_cli_name {
        //                         connection_config: Some(crate::common::ConnectionConfig::#ident),//?????????????
        //                     };
        //                     let new_context = #select_server_context::from_previous_context(()/* context: ()  подставить нужный */, new_context_scope).into();
        //                     Some(Self::#ident(
        //                         #ty::from(Some(cli_args), &new_context).ok()?,
        //                     ))
        //                 }
        //             }
        //         },
        //         _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
        //     }
            
        // });

         return quote! (); 

        // return quote! {
        //     pub fn from(
        //         optional_clap_variant: Option<#cli_name>,
        //         context: crate::common::Context, // заменить
        //     ) -> color_eyre::eyre::Result<Self> {
        //         match optional_clap_variant.and_then(|clap_variant| match clap_variant {
        //             #( #from_cli_enum_variants, )*
        //         }) {
        //             Some(x) => {Ok(x)}
        //             None => {Self::choose_variant(context)}
        //         }
        //     }
        // };
    };

    let from_cli_variants = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        match &variant.fields {
            syn::Fields::Unnamed(fields) => {
                let ty = &fields.unnamed[0].ty;
                quote! {
                    #cli_name::#variant_ident(args) => Some(Self::#variant_ident(#ty::from(Some(args), context.clone()).ok()?,)),
                }
            },
            _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
        }
        
    });
    
    quote! {
        pub fn from(
            optional_clap_variant: Option<#cli_name>,
            context: #context_dir, // Заменить
        ) -> color_eyre::eyre::Result<Self> {
            match optional_clap_variant.and_then(|clap_variant| match clap_variant {
                #(#from_cli_variants)*
            }) {
                Some(variant) => Ok(variant),
                None => Self::choose_variant(context.clone()),
            }                 
        }
    }
}
