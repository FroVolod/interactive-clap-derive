extern crate proc_macro;

use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;


pub fn choose_variant(ast: &syn::DeriveInput, variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let command_discriminants = syn::Ident::new(&format!("{}Discriminants", name), Span::call_site());
    let cli_command = syn::Ident::new(&format!("Cli{}", name), Span::call_site());

    let variant_ident = &variants[0].ident;
    let mut cli_variant = quote! ();

    for attr in &ast.attrs {
        if attr.path.is_ident("interactive_clap".into()) {
            for attr_token in attr.tokens.clone() {
                match attr_token {
                    proc_macro2::TokenTree::Group(group) => {
                        if &group.stream().to_string() == "disable_strum_discriminants" {
                            cli_variant = quote! {
                                let cli_variant = #cli_command::#variant_ident(Default::default());
                            };
                            break;
                        };
                    }
                    _ => () //abort_call_site!("Only option `TokenTree::Group` is needed")
                }
            }
            break;
        };
        if attr.path.is_ident("strum_discriminants".into()) {
            for attr_token in attr.tokens.clone() {
                match attr_token {
                    proc_macro2::TokenTree::Group(group) => {
                        if &group.stream().to_string() == "derive(EnumMessage, EnumIter)" {
                            let doc_attrs = ast.attrs.iter()
                                .filter(|attr| attr.path.is_ident("doc".into()))
                                .map(|attr| {
                                    let mut literal_string = String::new();
                                        for attr_token in attr.tokens.clone() {
                                            match attr_token {
                                                proc_macro2::TokenTree::Literal(literal) => {
                                                    literal_string = literal.to_string();
                                                }
                                                _ => () //abort_call_site!("Only option `TokenTree::Literal` is needed")
                                            }
                                        };
                                    literal_string
                                })
                                .collect::<Vec<_>>();
                            let literal_vec = doc_attrs.iter().map(|s| s.replace("\"", "")).collect::<Vec<_>>();
                            let literal = proc_macro2::Literal::string(literal_vec.join("\n  ").as_str());

                            let enum_variants = variants.iter().map(|variant| {
                                let variant_ident = &variant.ident;
                                quote! {
                                    #command_discriminants::#variant_ident => #cli_command::#variant_ident(Default::default())
                                }
                            });
                            
                            cli_variant = quote! {
                                let cli_variant = match crate::common::prompt_variant(#literal.to_string().as_str()) {
                                    #( #enum_variants, )*
                                };                                
                            };
                        };
                    }
                    _ => () //abort_call_site!("Only option `TokenTree::Group` is needed")
                }
            }
        };
    };

    quote! {
        #cli_variant
        Ok(Self::from(cli_variant, context)?)
    }
}
