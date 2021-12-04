extern crate proc_macro;

use std::net::SocketAddr;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::{ToTokens, __private::ext::RepToTokensExt, quote};

mod choose_variant;
mod from_cli_enum;


pub fn impl_interactive_clap(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let cli_name_string = format!("Cli{}", &ast.ident);
    let cli_name = &syn::Ident::new(&cli_name_string, Span::call_site());
    match &ast.data {
        syn::Data::Struct(data_struct) => {

            
            

            let fields = data_struct.fields.clone();
            let mut ident_skip_field_vec: Vec<syn::Ident> = vec![];

            let cli_fields = fields.iter().map(|field| {
                let ident_field = field.ident.clone().expect("this field does not exist");
                let ty = &field.ty;
                let mut cli_field = cli_field(&ident_field, ty);
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
                                        let enum_for_clap_named_arg = syn::Ident::new(&format!("ClapNamedArg{}For{}", &type_string, &name), Span::call_site());
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

            let from_cli_fields = fields.iter().map(|field| {
                from_cli_field(ast, field)                
            })
            .filter(|token_stream| !token_stream.is_empty());

            let context_scope_fields = fields.iter().map(|field| {
                let ident_field = &field.ident.clone().expect("this field does not exist");
                let ty = &field.ty;
                if field.attrs.is_empty() {
                    quote! {
                        pub #ident_field: #ty
                    }
                } else {
                    match field.attrs.iter()
                    .filter(|attr| attr.path.is_ident("interactive_clap".into()))
                    .map(|attr| attr.tokens.clone())
                    .flatten()
                    .filter(|attr_token| {
                        match attr_token {
                            proc_macro2::TokenTree::Group(group) => {
                                if group.stream().to_string().contains("subcommand") | group.stream().to_string().contains("named_arg") | group.stream().to_string().contains("skip") {
                                    false
                                } else {
                                    true
                                }
                            },
                            _ => abort_call_site!("Only option `TokenTree::Group` is needed")
                        }
                    })
                    .map(|_| {
                        quote! {
                            pub #ident_field: #ty
                        }
                    })
                    .next() {
                        Some(token_stream) => token_stream,
                        None => quote! ()
                    }
                }
                
            })
            .filter(|token_stream| !token_stream.is_empty())
            .collect::<Vec<_>>();
            let context_scope_for_struct = context_scope_for_struct(&name, context_scope_fields);

            let clap_enum_for_named_arg =
                if let Some(token_stream) = fields.iter().find_map(|field| {
                    let ident_field = &field.clone().ident.expect("this field does not exist");
                    let variant_name_string = crate::helpers::snake_case_to_camel_case::snake_case_to_camel_case(ident_field.to_string());
                    let variant_name = &syn::Ident::new(&variant_name_string, Span::call_site());
                    let attr_doc_vec: Vec<_> = field.attrs.iter()
                        .filter(|attr| attr.path.is_ident("doc".into()))
                        .map(|attr| attr.into_token_stream())
                        .collect();
                    
                    field.attrs.iter()
                        .filter(|attr| attr.path.is_ident("interactive_clap".into()))
                        .map(|attr| attr.tokens.clone())
                        .flatten()
                        .filter(|attr_token| {
                            match attr_token {
                                proc_macro2::TokenTree::Group(group) => {
                                    if group.stream().to_string().contains("named_arg") {
                                        true
                                    } else {
                                        false
                                    }
                                },
                                _ => abort_call_site!("Only option `TokenTree::Group` is needed")
                            }
                        })
                        .map(|_| {
                            let ty = &field.ty;
                            let type_string = match ty {
                                syn::Type::Path(type_path) => {
                                    match type_path.path.segments.last() {
                                        Some(path_segment) => path_segment.ident.to_string(),
                                        _ => String::new()
                                    }
                                },
                                _ => String::new()
                            };
                            let enum_for_clap_named_arg = syn::Ident::new(&format!("ClapNamedArg{}For{}", &type_string, &name), Span::call_site());
                            quote! {
                                #[derive(Debug, Clone, clap::Clap, interactive_clap_derive::ToCliArgs)]
                                pub enum #enum_for_clap_named_arg {
                                    #(#attr_doc_vec)*
                                    #variant_name(<#ty as ToCli>::CliVariant)
                                }

                                impl From<#ty> for #enum_for_clap_named_arg {
                                    fn from(item: #ty) -> Self {
                                        Self::#variant_name(<#ty as ToCli>::CliVariant::from(item))
                                    }
                                }
                            }
                        })
                        .next()
                }) {
                    token_stream
                } else {
                    quote! ()
                };

            let gen = quote! {
                #[derive(Debug, Default, Clone, clap::Clap, interactive_clap_derive::ToCliArgs)]
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

                #context_scope_for_struct

                //--------------------------------

                // impl #name {
                //     pub fn from(
                //         optional_clap_variant: Option<#cli_name>,
                //         context: crate::common::Context,
                //     ) -> color_eyre::eyre::Result<Self> {

                //         Ok(Self {
                //             #( #from_cli_fields, )*
                //         })
                //     }
                // }

                //--------------------------------

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

            let scope_for_enum = context_scope_for_enum(name);

            let fn_choose_variant = self::choose_variant::fn_choose_variant(ast, variants);

            let fn_from_cli = self::from_cli_enum::fn_from_cli(ast, variants);

            let gen = quote! {
                #[derive(Debug, Clone, clap::Clap, interactive_clap_derive::ToCliArgs)]
                pub enum #cli_name {
                    #( #enum_variants, )*
                }

                impl interactive_clap::ToCli for #name {
                    type CliVariant = #cli_name;
                }

                #scope_for_enum
                
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

fn context_scope_for_struct(name: &syn::Ident, context_scope_fields: Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let interactive_clap_context_scope_for_struct = syn::Ident::new(&format!("InteractiveClapContextScopeFor{}", &name), Span::call_site());
    quote! {
        pub struct #interactive_clap_context_scope_for_struct {
            #(#context_scope_fields,)*
        }
        impl interactive_clap::ToInteractiveClapContextScope for #name {
            type InteractiveClapContextScope = #interactive_clap_context_scope_for_struct;
        }
    }
}

fn context_scope_for_enum(name: &syn::Ident) -> proc_macro2::TokenStream {
    let interactive_clap_context_scope_for_enum = syn::Ident::new(&format!("InteractiveClapContextScopeFor{}", &name), Span::call_site());
    let enum_discriminants = syn::Ident::new(&format!("{}Discriminants", &name), Span::call_site());
    quote! {
        pub type #interactive_clap_context_scope_for_enum = #enum_discriminants;
        impl interactive_clap::ToInteractiveClapContextScope for #name {
                    type InteractiveClapContextScope = #interactive_clap_context_scope_for_enum;
                }
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

fn from_cli_field(ast: &syn::DeriveInput, field: &syn::Field) -> proc_macro2::TokenStream {
    for attr in &ast.attrs {
                
        if attr.path.is_ident("interactive_clap".into()) {

            for attr_token in attr.tokens.clone() {
                match attr_token {
                    proc_macro2::TokenTree::Group(group) => {
                        if group.to_string().contains("context") {
                            let ident_context = &group.stream().to_string();
                            let ident_context_vec: Vec<&str> = ident_context
                                .split(",")
                                .map(|s| s.trim())
                                .collect();
                            println!("++++++++  ident_context: {:#?}", ident_context_vec);

                            for item in group.stream() {
                                match item {
                                    proc_macro2::TokenTree::Ident(ident) => {
                                        
                                        if "input_context".to_string() == ident.to_string() {
                                            let input_context = &group.stream().to_string();
                                            let input_context_vec: Vec<&str> = input_context
                                                .split("input_context")
                                                .collect();
                                            println!("---------  input_context: {:#?}", input_context_vec);
                                        }
                                        if "output_context".to_string() == ident.to_string() {
                                            println!("---------  output_context: {:#?}", &group.to_string().split_once("output_context").unwrap());
                                        }
                                        if "context".to_string() == ident.to_string() {
                                            println!("---------  context: {:#?}", &group.to_string().split_once("context").unwrap());
                                        }

                                    },
                                    _ => () //abort_call_site!("Only option `TokenTree::Ident` is needed")
                                };
                            };
                        }

                        
                    },
                    _ => abort_call_site!("Only option `TokenTree::Group` is needed")
                }
            };

        };
    };

    // не subcommand
    quote! {
        allowance: match optional_clap_variant
            .clone()
            .and_then(|clap_variant| clap_variant.allowance)
        {
            Some(cli_allowance) => Some(cli_allowance.to_yoctonear()),
            None => FunctionCallType::input_allowance(),
        };
    };

    // subcommand с неизмененным contet
    quote! {
        currency_selection: match optional_clap_variant.and_then(|clap_variant| clap_variant.currency_selection) {
            Some(cli_currency_selection) => {
                CurrencySelection::from(Some(cli_currency_selection), context)?
            }
            None => CurrencySelection::choose_variant(context)?,
        }
    };

    // subcommand с изменееным context
    quote! {
        public_key_mode: {
            type Alias = <Sender as crate::common::ToInteractiveClapContextScope>::InteractiveClapContextScope;
            let new_context_scope = Alias {
                sender_account_id
            };
            let new_context /*: SignerContext */ = SenderContext::from_previous_context(context, &new_context_scope);
            let public_key_mode = super::public_key_mode::PublicKeyMode::from(
                optional_clap_variant.and_then(|clap_variant| clap_variant.public_key_mode),
                &new_context,
            )?
        }
    };

    quote! {}
    


}
