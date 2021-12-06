extern crate proc_macro;

use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;


pub fn from_cli_for_enum(ast: &syn::DeriveInput, variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let cli_name = syn::Ident::new(&format!("Cli{}", name), Span::call_site());

    let mut context_dir = quote! ();
    let mut input_context_dir = quote! ();
    let mut output_context_dir = quote! ();
    let mut is_fn_from_default = false;
    
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
                        if group.stream().to_string().contains("fn_from_cli") && group.stream().to_string().contains("default") {
                            is_fn_from_default = true;
                        };
                    }
                    _ => () //abort_call_site!("Only option `TokenTree::Group` is needed")
                }
            }
        };
    };

    if is_fn_from_default {
         return quote! (); 
    };

    let from_cli_variants = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        match &variant.fields {
            syn::Fields::Unnamed(fields) => {
                let ty = &fields.unnamed[0].ty;
                let context_name = syn::Ident::new(&format!("{}Context", &name), Span::call_site());
                if output_context_dir.is_empty() {
                    quote! {
                        Some(#cli_name::#variant_ident(args)) => Ok(Self::#variant_ident(#ty::from(Some(args), context.clone())?,)),
                    }
                } else {
                    quote! {
                        Some(#cli_name::#variant_ident(args)) => {
                            type Alias = <#name as ToInteractiveClapContextScope>::InteractiveClapContextScope;
                            let new_context_scope = Alias::#variant_ident;
                            let new_context = #context_name::from_previous_context((), &new_context_scope);
                            Ok(Self::#variant_ident(#ty::from(Some(args), new_context)?,))
                        }
                    }
                }
            },
            _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
        }
        
    });

    let input_context = if let true = !context_dir.is_empty() {
        context_dir
    } else {
        input_context_dir
    };
    
    quote! {
        pub fn from(
            optional_clap_variant: Option<#cli_name>,
            context: #input_context,
        ) -> color_eyre::eyre::Result<Self> {
            match optional_clap_variant {
                #(#from_cli_variants)*
                None => Self::choose_variant(context.clone()),                             
            }
        }
    }
}
