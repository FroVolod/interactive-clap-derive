extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::{ToTokens,  quote};

mod choose_variant;
mod from_cli_enum;


pub fn impl_interactive_clap(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let cli_name_string = format!("Cli{}", &ast.ident);
    let cli_name = &syn::Ident::new(&cli_name_string, Span::call_site());
    match &ast.data {
        syn::Data::Struct(data_struct) => {
            let fields = data_struct.clone().fields;
            let mut clap_enum_for_named_arg = quote! ();
            let mut ident_skip_field_vec: Vec<syn::Ident> = vec![];

            let cli_fields = fields.iter().map(|field| {
                let ident_field = &field.clone().ident.expect("this field does not exist");
                let ty = &field.ty;
                let mut cli_field = cli_field(ident_field, ty);
                if field.attrs.is_empty() {
                    return cli_field
                };
                let mut clap_attr_vec: Vec<_> = vec![];
                for attr in &field.attrs {
                    if attr.path.is_ident("interactive_clap".into()) {
                        for attr_token in attr.tokens.clone() {
                            match attr_token {
                                proc_macro2::TokenTree::Group(group) => {
                                    if group.stream().to_string().contains("subcommand") | group.stream().to_string().contains("long") | group.stream().to_string().contains("skip") {
                                        clap_attr_vec.push(group.stream())
                                    } else if group.stream().to_string().contains("named_arg") {
                                        let ident_subcommand = syn::Ident::new("subcommand", Span::call_site());
                                        clap_attr_vec.push(quote! {#ident_subcommand});
                                        let type_string = match ty {
                                            syn::Type::Path(type_path) => {
                                                match type_path.path.segments.last() {
                                                    Some(path_segment) => path_segment.ident.to_string(),
                                                    _ => String::new()
                                                }
                                            },
                                            _ => String::new()
                                        };
                                        let enum_for_clap_named_arg = syn::Ident::new(&format!("ClapNamedArg{}", &type_string), Span::call_site());
                                        cli_field = quote! {
                                            pub #ident_field: Option<#enum_for_clap_named_arg>
                                        }
                                    } else {
                                        continue;
                                    };
                                    if group.stream().to_string().contains("skip") {
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
                    let clap_attrs = clap_attr_vec.iter();
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

            for field in fields.iter() {
                let mut attr_doc_vec = vec![quote! ()];
                let mut attr_interactive_clap_vec = vec![quote! ()];
                let ty = &field.ty;
                for attr in &field.attrs {
                    if attr.path.is_ident("doc".into()) {
                        attr_doc_vec.push(attr.into_token_stream());
                        continue;
                    };
                    if attr.path.is_ident("interactive_clap".into()) {
                        attr_interactive_clap_vec.push(attr.tokens.clone());
                        
                    };
                };
                for attr_interactive_clap in attr_interactive_clap_vec {
                    for attr_token in attr_interactive_clap {
                        match attr_token {
                            proc_macro2::TokenTree::Group(group) => {
                                if group.stream().to_string().contains("named_arg") {
                                    let type_string = match ty {
                                        syn::Type::Path(type_path) => {
                                            match type_path.path.segments.last() {
                                                Some(path_segment) => path_segment.ident.to_string(),
                                                _ => String::new()
                                            }
                                        },
                                        _ => String::new()
                                    };
                                    let enum_for_clap_named_arg = syn::Ident::new(&format!("ClapNamedArg{}", &type_string), Span::call_site());
                                    let field_for_clap_named_arg = syn::Ident::new(&type_string, Span::call_site());
                                    if  !group.stream().to_string().contains("duplicate") {
                                        clap_enum_for_named_arg = quote! {
                                            #[derive(Debug, Clone, clap::Clap, ToCliArgs)]
                                            pub enum #enum_for_clap_named_arg {
                                                #(#attr_doc_vec)*
                                                #field_for_clap_named_arg(<#ty as ToCli>::CliVariant)
                                            }

                                            impl From<#ty> for #enum_for_clap_named_arg {
                                                fn from(item: #ty) -> Self {
                                                    Self::#field_for_clap_named_arg(<#ty as ToCli>::CliVariant::from(item))
                                                }
                                            }
                                        };
                                    } else {
                                        clap_enum_for_named_arg = quote! ();
                                    };
                                };
                            },
                            _ => abort_call_site!("Only option `TokenTree::Group` is needed")
                        }
                    };
                };
            };

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

                #clap_enum_for_named_arg
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

            let fn_choose_variant = self::choose_variant::fn_choose_variant(ast, variants);

            let fn_from_cli = self::from_cli_enum::fn_from_cli(ast, variants);

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
                    #fn_choose_variant
                    #fn_from_cli
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
                                    pub #ident_field: Option<<#ty_option as ToCli>::CliVariant>
                                }
                            },
                            _ => {
                                quote! {
                                    pub #ident_field: Option<<#ty as ToCli>::CliVariant>
                                }
                            },
                        }
                    } else {
                        quote! {
                            pub #ident_field: Option<<#ty as ToCli>::CliVariant>
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
