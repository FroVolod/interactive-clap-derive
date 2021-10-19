extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::{ToTokens,  quote};

mod choose_variant;


pub fn impl_interactive_clap(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let cli_name_string = format!("Cli{}", &ast.ident);
    let cli_name = &syn::Ident::new(&cli_name_string, Span::call_site());
    match &ast.data {
        syn::Data::Struct(data_struct) => {
            let fields = data_struct.clone().fields;
            let mut ident_skip_field_vec: Vec<syn::Ident> = vec![];
            let cli_fields = fields.iter().map(|field| {
                let ident_field = &field.clone().ident.expect("this field does not exist");
                let ty = &field.ty;
                let mut cli_field = cli_field(ident_field, ty);
                if field.attrs.is_empty() {
                    return cli_field
                };
                let mut clap_attr_vec: Vec<String> = vec![];
                for attr in &field.attrs {
                    if attr.path.is_ident("interactive_clap".into()) {
                        for attr_token in attr.tokens.clone() {
                            match attr_token {
                                proc_macro2::TokenTree::Group(group) => {
                                    if ["subcommand", "long", "skip"].contains(&group.stream().to_string().as_str()) {
                                        clap_attr_vec.push(group.stream().to_string())
                                    } else {
                                        continue;
                                    };
                                    if "skip" == &group.stream().to_string() {
                                        ident_skip_field_vec.push(ident_field.clone());
                                        cli_field = quote! ()
                                    };
                                },
                                _ => abort_call_site!("Only option `TokenTree::Group` is needed")
                            }
                        };
                    }
                };
                if cli_field.is_empty() {
                    return cli_field
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
            })
            .filter(|token_stream| !token_stream.is_empty())
            .collect::<Vec<_>>();

            let for_cli_fields = fields.iter().map(|field| {
                for_cli_field(field, &ident_skip_field_vec)                
            })
            .filter(|token_stream| !token_stream.is_empty());

            let gen = quote! {
                #[derive(Debug, Default, Clone, clap::Clap, ToCliArgs)]
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
            let choose_variant = choose_variant::choose_variant(ast, variants);


            let gen = quote! {
                #[derive(Debug, Clone, clap::Clap, ToCliArgs)]
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

                impl #name {
                    pub fn choose_variant(context: crate::common::Context) -> color_eyre::eyre::Result<Self> {
                        #choose_variant
                    } 
                }
            };
            gen.into()
        }
        _ => abort_call_site!("`#[derive(InteractiveClap)]` only supports structs and enums")
    }
}

fn cli_field(ident_field: &syn::Ident, ty: &syn::Type) -> proc_macro2::TokenStream {
    match &ty {
        syn::Type::Path(type_path) => {
            match type_path.path.segments.first() {
                Some(path_segment) => {
                    if path_segment.ident.eq("Option".into()) {
                        match &path_segment.arguments {
                            syn::PathArguments::AngleBracketed(gen_args) => {
                                let ty_option = &gen_args.args;
                                quote! {
                                    #ident_field: Option<<#ty_option as ToCli>::CliVariant>
                                }
                            },
                            _ => {
                                quote! {
                                    #ident_field: Option<<#ty as ToCli>::CliVariant>
                                }
                            },
                        }
                    } else {
                        quote! {
                            #ident_field: Option<<#ty as ToCli>::CliVariant>
                        }
                    }
                },
                _ => abort_call_site!("Only option `PathSegment` is needed")
            }
        },
        _ => abort_call_site!("Only option `Type::Path` is needed")
    }
}

fn for_cli_field(field: &syn::Field, ident_skip_field_vec: &Vec<syn::Ident>) -> proc_macro2::TokenStream {
    let ident_field = &field.clone().ident.expect("this field does not exist");
    if ident_skip_field_vec.contains(&ident_field) {
        quote! ()
    } else {
        let ty = &field.ty;
        match &ty {
            syn::Type::Path(type_path) => {
                match type_path.path.segments.first() {
                    Some(path_segment) => {
                        if path_segment.ident.eq("Option".into()) {
                            quote! {
                                #ident_field: args.#ident_field.into()
                            }
                        } else {
                            quote! {
                                #ident_field: Some(args.#ident_field.into())
                            }
                        }
                    },
                    _ => abort_call_site!("Only option `PathSegment` is needed")
            }},
            _ => abort_call_site!("Only option `Type::Path` is needed")
        }
    }
}
