extern crate proc_macro;

use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;


pub fn from_cli_for_struct(ast: &syn::DeriveInput, fields: &syn::Fields) -> proc_macro2::TokenStream {
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

    let fields_without_subcommand = fields.iter().map(|field| {
        field_without_subcommand(field)                
    })
    .filter(|token_stream| !token_stream.is_empty())
    .collect::<Vec<_>>();

    let fields_value = fields.iter().map(|field| {
        fields_value(field)                
    })
    .filter(|token_stream| !token_stream.is_empty());

    let field_value_named_arg = 
        if let Some(token_stream) = fields.iter().map(|field| {
            field_value_named_arg(name, field, &output_context_dir)                
        })
        .filter(|token_stream| !token_stream.is_empty())
        .next()
        {
            token_stream
        } else {
            quote! ()
        };

    let field_value_subcommand = 
        if let Some(token_stream) = fields.iter().map(|field| {
            field_value_subcommand(name, field, &output_context_dir)                
        })
        .filter(|token_stream| !token_stream.is_empty())
        .next()
        {
            token_stream
        } else {
            quote! ()
        };

    let struct_fields = fields.iter().map(|field| {
        struct_field(field, &fields_without_subcommand)                
    });

    let input_context = if let true = !context_dir.is_empty() {
        context_dir
    } else {
        input_context_dir
    };

    let interactive_clap_context_scope_for_struct = syn::Ident::new(&format!("InteractiveClapContextScopeFor{}", &name), Span::call_site());
    let new_context_scope = quote! {
        let new_context_scope = #interactive_clap_context_scope_for_struct { #(#fields_without_subcommand,)* };
    };
    
    quote! {
        pub fn from(
            optional_clap_variant: Option<#cli_name>,
            context: #input_context,
        ) -> color_eyre::eyre::Result<Self> {
            #(#fields_value)*
            #new_context_scope
            #field_value_named_arg
            #field_value_subcommand
            Ok(Self{ #(#struct_fields,)* })
        }
    }
}

fn field_without_subcommand(field: &syn::Field) -> proc_macro2::TokenStream {
    let ident_field = &field.clone().ident.expect("this field does not exist");
    if field.attrs.is_empty() {
        quote! {#ident_field}
    } else {
        match field.attrs.iter()
        .filter(|attr| attr.path.is_ident("interactive_clap".into()))
        .map(|attr| attr.tokens.clone())
        .flatten()
        .filter(|attr_token| {
            match attr_token {
                proc_macro2::TokenTree::Group(group) => {
                    if group.stream().to_string().contains("named_arg") || group.stream().to_string().contains("subcommand") {
                        false
                    } else {
                        true
                    }
                },
                _ => abort_call_site!("Only option `TokenTree::Group` is needed")
            }
        })
        .map(|_| {
            quote! {#ident_field}
        })
        .next() {
            Some(token_stream) => token_stream,
            None => quote! ()
        }
    }
}

fn fields_value(field: &syn::Field) -> proc_macro2::TokenStream {
    let ident_field = &field.clone().ident.expect("this field does not exist");
    let fn_input_arg = syn::Ident::new(&format!("input_{}", &ident_field), Span::call_site());
    if field.attrs.is_empty() {
        quote! {
            let #ident_field = match optional_clap_variant
                .clone()
                .and_then(|clap_variant| clap_variant.#ident_field)
            {
                Some(#ident_field) => #ident_field,
                None => Self::#fn_input_arg(&context)?,
            };
        }
        
    } else {
        match field.attrs.iter()
        .filter(|attr| attr.path.is_ident("interactive_clap".into()))
        .map(|attr| attr.tokens.clone())
        .flatten()
        .filter(|attr_token| {
            match attr_token {
                proc_macro2::TokenTree::Group(group) => {
                    if group.stream().to_string().contains("named_arg") || group.stream().to_string().contains("subcommand") {
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
                let #ident_field = match optional_clap_variant
                    .clone()
                    .and_then(|clap_variant| clap_variant.#ident_field)
                {
                    Some(#ident_field) => #ident_field,
                    None => Self::#fn_input_arg(&context)?,
                };
            }
        })
        .next() {
            Some(token_stream) => token_stream,
            None => quote! ()
        }
    }
}

fn field_value_named_arg(name: &syn::Ident, field: &syn::Field, output_context_dir: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let ident_field = &field.clone().ident.expect("this field does not exist");
    let ty = &field.ty;
    if field.attrs.is_empty() {
        quote! ()
    } else {
        match field.attrs.iter()
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
            let variant_name_string = crate::helpers::snake_case_to_camel_case::snake_case_to_camel_case(ident_field.to_string());
            let variant_name = &syn::Ident::new(&variant_name_string, Span::call_site());
            if output_context_dir.is_empty() {
                quote! {
                    let #ident_field = #ty::from(
                        optional_clap_variant.and_then(|clap_variant| match clap_variant.#ident_field {
                            Some(#enum_for_clap_named_arg::#variant_name(cli_sender)) => Some(cli_sender),
                            None => None,
                        }),
                        context.into(),
                    )?;
                }
            } else {
                let context_for_struct = syn::Ident::new(&format!("{}Context", &name), Span::call_site());
                quote! {
                    let new_context = #context_for_struct::from_previous_context(context, &new_context_scope);
                    let #ident_field = #ty::from(
                        optional_clap_variant.and_then(|clap_variant| match clap_variant.#ident_field {
                            Some(#enum_for_clap_named_arg::#variant_name(cli_arg)) => Some(cli_arg),
                            None => None,
                        }),
                        new_context.into(),
                    )?;
                }
            }
        })
        .next() {
            Some(token_stream) => token_stream,
            None => quote! ()
        }
    }
}

fn field_value_subcommand(name: &syn::Ident, field: &syn::Field, output_context_dir: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let ident_field = &field.clone().ident.expect("this field does not exist");
    let ty = &field.ty;
    if field.attrs.is_empty() {
        quote! ()
    } else {
        match field.attrs.iter()
        .filter(|attr| attr.path.is_ident("interactive_clap".into()))
        .map(|attr| attr.tokens.clone())
        .flatten()
        .filter(|attr_token| {
            match attr_token {
                proc_macro2::TokenTree::Group(group) => {
                    if group.stream().to_string().contains("subcommand") {
                        true
                    } else {
                        false
                    }
                },
                _ => abort_call_site!("Only option `TokenTree::Group` is needed")
            }
        })
        .map(|_| {
            if output_context_dir.is_empty() {
                quote! {
                    let #ident_field = match optional_clap_variant.and_then(|clap_variant| clap_variant.#ident_field) {
                        Some(cli_arg) => #ty::from(Some(cli_arg), context)?,
                        None => #ty::choose_variant(context)?,
                    };
                }
            } else {
                let context_for_struct = syn::Ident::new(&format!("{}Context", &name), Span::call_site());
                quote! {
                    let new_context = #context_for_struct::from_previous_context(context, &new_context_scope);
                    let #ident_field = match optional_clap_variant.and_then(|clap_variant| clap_variant.#ident_field) {
                        Some(cli_arg) => #ty::from(Some(cli_arg), new_context)?,
                        None => #ty::choose_variant(new_context)?,
                    };
                }
            }
        })
        .next() {
            Some(token_stream) => token_stream,
            None => quote! ()
        }
    }
}

fn struct_field(field: &syn::Field, fields_without_subcommand: &Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let ident_field = &field.clone().ident.expect("this field does not exist");
    let fields_without_subcommand_to_string = fields_without_subcommand.iter().map(|token_stream| token_stream.to_string()).collect::<Vec<_>>();
    if fields_without_subcommand_to_string.contains(&ident_field.to_string()) {
        quote! {
            #ident_field: new_context_scope.#ident_field
        }
    } else {
        quote! {
            #ident_field
        }
    }
}
