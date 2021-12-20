extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;


pub fn impl_to_cli_args(ast: &syn::DeriveInput) -> TokenStream {
    let cli_name = &ast.ident;
    match &ast.data {
        syn::Data::Struct(data_struct) => {
            let mut args_subcommand = quote! {
                let mut args = std::collections::VecDeque::new();
            };
            let mut args_push_front_vec = vec![quote!()];
            
            for field in data_struct.clone().fields.iter() {
                let ident_field = &field.ident.clone().expect("this field does not exist");
                if field.attrs.is_empty() {
                    let args_push_front = quote!{
                        if let Some(arg) = &self.#ident_field {
                            args.push_front(arg.to_string())
                        }
                    };
                    args_push_front_vec.push(args_push_front.clone());
                } else {
                    for attr in &field.attrs {
                        if attr.path.is_ident("clap".into()) {
                            for attr_token in attr.tokens.clone() {
                                match attr_token {
                                    proc_macro2::TokenTree::Group(group) => {
                                        for item in group.stream() {
                                            match &item {
                                                proc_macro2::TokenTree::Ident(ident) => {
                                                    if "subcommand".to_string() == ident.to_string() {
                                                        args_subcommand = quote! {
                                                            let mut args = self
                                                                .#ident_field
                                                                .as_ref()
                                                                .map(|subcommand| subcommand.to_cli_args())
                                                                .unwrap_or_default();
                                                        };
                                                    } else if "long".to_string() == ident.to_string() {
                                                        let ident_field_to_kebab_case_string = crate::helpers::to_kebab_case::to_kebab_case(ident_field.to_string());
                                                        let ident_field_to_kebab_case = &syn::LitStr::new(&ident_field_to_kebab_case_string, Span::call_site());
                                                        let args_push_front = quote!{
                                                            if let Some(arg) = &self.#ident_field {
                                                                args.push_front(arg.to_string());
                                                                args.push_front(std::concat!("--", #ident_field_to_kebab_case).to_string());
                                                            }
                                                        };
                                                        args_push_front_vec.push(args_push_front.clone());
                                                    }
                                                },
                                                proc_macro2::TokenTree::Literal(literal) =>{
                                                    let args_push_front = quote!{
                                                            if let Some(arg) = &self.#ident_field {
                                                                args.push_front(arg.to_string());
                                                                args.push_front(std::concat!("--", #literal).to_string());
                                                            }
                                                        };
                                                    args_push_front_vec.pop();
                                                    args_push_front_vec.push(args_push_front.clone());
                                                },
                                                _ => () //abort_call_site!("Only option `TokenTree::Ident` is needed")
                                            };
                                        };
                                    },
                                    _ => abort_call_site!("Only option `TokenTree::Group` is needed")
                                }
                            };
                        }
                    };
                };
            };
            let args_push_front_vec = args_push_front_vec.into_iter().rev();
            let gen = quote! {
                impl #cli_name {
                    pub fn to_cli_args(&self) -> std::collections::VecDeque<String> {
                        #args_subcommand;
                        #(#args_push_front_vec; )*
                        args
                    }
                }
            };
            gen.into()
        },
        syn::Data::Enum(syn::DataEnum { variants, .. }) => {
            let enum_variants = variants.iter().map(|variant| {
                let ident = &variant.ident;
                let variant_name_string = crate::helpers::to_kebab_case::to_kebab_case(ident.to_string());
                let variant_name = &syn::LitStr::new(&variant_name_string, Span::call_site());

                match &variant.fields {
                    syn::Fields::Unnamed(_) => {
                        quote! {
                            Self::#ident(subcommand) => {
                                let mut args = subcommand.to_cli_args();
                                args.push_front(#variant_name.to_owned());
                                args
                            }
                        }
                    },
                    syn::Fields::Unit => {
                        quote! {
                            Self::#ident => {
                                let mut args = std::collections::VecDeque::new();
                                args.push_front(#variant_name.to_owned());
                                args
                            }
                        }
                    },
                    _ => abort_call_site!("Only options `Fields::Unnamed` or `Fields::Unit` are needed")
                }

                
            });
            let gen = quote! {
                impl #cli_name {
                    pub fn to_cli_args(&self) -> std::collections::VecDeque<String> {
                        match self {
                            #( #enum_variants, )*
                        }
                    }
                }
            };
            gen.into()
        },
        _ => abort_call_site!("`#[derive(InteractiveClap)]` only supports structs and enums")
    }
}
