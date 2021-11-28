extern crate proc_macro;

use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;


pub fn fn_from_cli(ast: &syn::DeriveInput, variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let cli_name = syn::Ident::new(&format!("Cli{}", name), Span::call_site());

    let mut context_dir = quote! ();
    let mut input_context_dir = quote! ();
    let mut output_context_dir = quote! ();

    let mut is_fn_from_default = false;


    // let input_context_dir = ast.attrs
    //     .iter()
    //     .filter(|attr| attr.path.is_ident("interactive_clap"))
    //     .map(|attr| {
    //         let mut input_context_dir = quote! ();

    //         for token_tree in attr.tokens.clone() {
    //             match token_tree {
    //                 proc_macro2::TokenTree::Group(group) => {
    //                     if group.stream().to_string().contains("input_context") {
    //                         let group_stream = &group.stream()
    //                             .into_iter()
    //                             // .enumerate()
    //                             // // .filter(|&(i,_)| i != 0 || i != 1)
    //                             // .map(|(_, v)| v)
    //                             .collect::<Vec<_>>()[2..];
    //                         input_context_dir = quote! {#(#group_stream)*}
    //                     }
    //                 }
    //                 _ => () //abort_call_site!("Only option `TokenTree::Group` is needed")
    //             }
    //         };
    //         input_context_dir     
    //     })
    //     .filter(|token_stream| !token_stream.is_empty())
    //     .next()
    //     .expect("input_context does not exist");
    //     // .collect::<Vec<_>>();
    // // let input_context = &input_context[2..];
    // println!("!!!!!!!!!!!  input_context: {:#?}", &input_context_dir);

    
    for attr in ast.attrs.clone() {
        if attr.path.is_ident("interactive_clap".into()) {
            for attr_token in attr.tokens.clone() {
                match attr_token {
                    proc_macro2::TokenTree::Group(group) => {
                        if group.stream().to_string().contains("output_context") {
                            let group_stream = &group.stream()
                            .into_iter()
                            .collect::<Vec<_>>()[2..];
                            output_context_dir = quote! {#(#group_stream)*};
                        } else if group.stream().to_string().contains("input_context") {
                            let group_stream = &group.stream()
                            .into_iter()
                            .collect::<Vec<_>>()[2..];
                            input_context_dir = quote! {#(#group_stream)*};
                        } else if group.stream().to_string().contains("context") {
                            let group_stream = &group.stream()
                            .into_iter()
                            // .enumerate()
                            // .filter(|&(i,_)| i != 0 || i != 1)
                            // .map(|(_, v)| v)
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
                let context_name = syn::Ident::new(&format!("{}Context", &name), Span::call_site());
                match output_context_dir.is_empty() {
                    true => quote! {
                        #cli_name::#variant_ident(args) => Some(Self::#variant_ident(#ty::from(Some(args), context.clone()).ok()?,)),
                    },
                    false => quote! {
                        #cli_name::#variant_ident(args) => {
                            type Alias = <#name as crate::common::ToInteractiveClapContextScope>::InteractiveClapContextScope;
                            let new_context_scope = Alias::#variant_ident;
                            let new_context = #context_name::from_previous_context((), new_context_scope);
                            Some(Self::#variant_ident(#ty::from(Some(args), new_context.clone()).ok()?,))
                        }
                    }
                }
            },
            _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
        }
        
    });

    // let qwe = context_dir.is_empty();

    let input_context = if let true = !context_dir.is_empty() {
        context_dir
    } else {
        input_context_dir
    };
    
    // if !output_context_dir.is_empty() {
    //     println!("=  =  =  =  =  output_context_dir: {:#?}", &output_context_dir);
    // };
    quote! {
        pub fn from(
            optional_clap_variant: Option<#cli_name>,
            context: #input_context,
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

