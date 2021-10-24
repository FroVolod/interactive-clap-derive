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

    if is_fn_from_default { return quote! (); };

    let from_cli_variants = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        match &variant.fields {
            syn::Fields::Unnamed(fields) => {
                let ty = &fields.unnamed[0].ty;
                quote! {
                    #cli_name::#variant_ident(args) => Ok(Self::#variant_ident(#ty::from(args, context)?,)),
                }
            },
            _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
        }
        
    });
    
    quote! {
        pub fn from(
            item: #cli_name,
            context: #context_dir,
        ) -> color_eyre::eyre::Result<Self> {
            match item {
                #(#from_cli_variants)*
            }                    
        }
    }
}
