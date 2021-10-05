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
                let cli_field = match &ty {
                    syn::Type::Path(type_path) => {
                        match type_path.path.segments.first() {
                            Some(path_segment) => {
                                if path_segment.ident.eq("Option".into()) {
                                    match &path_segment.arguments {
                                        syn::PathArguments::AngleBracketed(gen_args) => {
                                            let ty_option = &gen_args.args;
                                            quote! {
                                                #ident: Option<<#ty_option as ToCli>::CliVariant>
                                            }
                                        },
                                        _ => {
                                            quote! {
                                                #ident: Option<<#ty as ToCli>::CliVariant>
                                            }
                                        },
                                    }
                                } else {
                                    quote! {
                                        #ident: Option<<#ty as ToCli>::CliVariant>
                                    }
                                }
                            },
                            _ => quote! {
                                    #ident: Option<<#ty as ToCli>::CliVariant>
                                }
                        }
                    },
                    _ => quote! {
                            #ident: Option<<#ty as ToCli>::CliVariant>
                        }
                };
                if field.attrs.is_empty() {
                    return cli_field
                };
                let mut clap_attr_vec: Vec<String> = vec![];
                for attr in &field.attrs {
                    if attr.path.is_ident("interactive_clap".into()) {
                        for attr_token in attr.tokens.clone() {
                            match attr_token {
                                proc_macro2::TokenTree::Group(group) => {
                                    for item in group.stream() {
                                        match item {
                                            proc_macro2::TokenTree::Ident(ident) => {
                                                if ["subcommand", "long", "skip"].contains(&ident.to_string().as_str()) {
                                                    clap_attr_vec.push(ident.to_string())
                                                }
                                            },
                                            _ => ()
                                        };
                                    }
                                },
                                _ => ()
                            }
                        };
                    }
                };
                if !clap_attr_vec.is_empty() {
                    let clap_attrs = clap_attr_vec.iter().map(|clap_attr| {
                        let attr = &syn::Ident::new(clap_attr, Span::call_site());
                        quote! {#attr}
                    });
                    quote! {
                        #[clap(#(#clap_attrs, )*)]
                        #cli_field
                    }
                } else {
                    quote! {
                        #cli_field
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

                impl interactive_clap::ToCli for #name {
                    type CliVariant = #cli_name;
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
            let for_cli_enum_variants = variants.iter().map(|variant| {
                let ident = &variant.ident;

                quote! { #name::#ident(arg) => Self::#ident(arg.into()) }
            });
            let gen = quote! {
                #[derive(Debug, Clone, clap::Clap)]
                pub enum #cli_name {
                    #( #enum_variants, )*
                }

                impl interactive_clap::ToCli for #name {
                    type CliVariant = #cli_name;
                }

                impl From<#name> for #cli_name {
                    fn from(command: #name) -> Self {
                        match command {
                            #( #for_cli_enum_variants, )*
                        }
                    }
                }
            };
            gen.into()
        }
        _ => abort_call_site!("`#[derive(InteractiveClap)]` only supports structs and enums")
    }
}
