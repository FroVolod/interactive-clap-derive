extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::{ToTokens,  quote};


#[proc_macro_derive(InteractiveClap, attributes(interactive_clap))]
pub fn interactive_clap(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);
    impl_interactive_clap(&ast)
}

fn impl_interactive_clap(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let cli_name_string = format!("Cli{}", &ast.ident);
    let cli_name = &syn::Ident::new(&cli_name_string, Span::call_site());
    
    match &ast.data {
        syn::Data::Struct(data_struct) => {
            let fields = data_struct.clone().fields;
            let cli_fields = fields.iter().map(|field| {
                let ident = &field.clone().ident.expect("this field does not exist");
                let ty = &field.ty;
                if field.attrs.is_empty() {
                    return quote! {
                        #ident: Option<<#ty as ToCli>::CliVariant>
                    }
                };
                let is_attr_interactive_clap_subcommand: bool = {
                    let mut is_attr_interactive_clap: bool = false;
                    for attr in &field.attrs {
                        if attr.path.is_ident("interactive_clap".into()) {
                            for attr_token in attr.tokens.clone() {
                                if match attr_token {
                                    proc_macro2::TokenTree::Group(group) => {
                                        for item in group.stream() {
                                            is_attr_interactive_clap = match item {
                                                // checking the format of the attribute #[interactive_clap(subcommand)]
                                                proc_macro2::TokenTree::Ident(ident) => {
                                                    if &ident.to_string() == "subcommand" {
                                                        true
                                                    } else {
                                                        false
                                                    }
                                                },
                                                _ => false
                                            };
                                            if is_attr_interactive_clap {break;}
                                        };
                                        is_attr_interactive_clap
                                    },
                                    _ => false
                                }
                                {break;}
                            };
                        }
                    };
                    is_attr_interactive_clap
                };
                if is_attr_interactive_clap_subcommand {
                    quote! {
                        #[clap(subcommand)]
                        #ident: Option<<#ty as ToCli>::CliVariant>
                    }
                } else {
                    quote! {
                        #ident: Option<<#ty as ToCli>::CliVariant>
                    }
                }
            });
            let for_cli_fields = fields.iter().map(|field| {
                let ident = &field.clone().ident.expect("this field does not exist");
                quote! {
                    #ident: Some(args.#ident.into())
                }
            });

            let gen = quote! {
                #[derive(Debug, Default, Clone, clap::Clap)]
                #[clap(
                    setting(clap::AppSettings::ColoredHelp),
                    setting(clap::AppSettings::DisableHelpSubcommand),
                    setting(clap::AppSettings::VersionlessSubcommands)
                )]
                pub struct #cli_name {
                    #( #cli_fields, )*
                }

                impl From<#name> for #cli_name {
                    fn from(args: #name) -> Self {
                        Self {
                            #( #for_cli_fields, )*
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Enum(syn::DataEnum { variants, .. }) => {
            let enum_variants = variants.iter().map(|variant| {
                let ident = &variant.ident;
                let mut attrs: Vec<proc_macro2::TokenStream> = vec![];
                if !&variant.attrs.is_empty() {
                    for attr in &variant.attrs {
                        if attr.path.is_ident("doc".into()) {
                            attrs.push(attr.into_token_stream()) ;
                            break;
                        };
                    };
                    match &variant.fields {
                        syn::Fields::Unnamed(fields) => {
                            let ty = &fields.unnamed[0].ty;
                            if attrs.is_empty() {
                                quote! {#ident(<#ty as ToCli>::CliVariant)}
                            } else {
                                let attr = attrs.iter().next().unwrap();
                                quote! {
                                    #attr
                                    #ident(<#ty as ToCli>::CliVariant)
                                }
                            }
                        },
                        _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
                    }
                } else {
                    match &variant.fields {
                        syn::Fields::Unnamed(fields) => {
                            let ty = &fields.unnamed[0].ty;
                            quote! { #ident(<#ty as ToCli>::CliVariant) }
                            
                        },
                        _ => abort_call_site!("Only option `Fields::Unnamed` is needed")
                    }
                }
            });
            let gen = quote! {
                #[derive(Debug, Clone, clap::Clap)]
                pub enum #cli_name {
                    #( #enum_variants, )*
                }

                impl interactive_clap::ToCli for #name {
                    type CliVariant = #cli_name;
                }
            };
            gen.into()
        }
        _ => abort_call_site!("`#[derive(InteractiveClap)]` only supports structs and enums")
    }
}
