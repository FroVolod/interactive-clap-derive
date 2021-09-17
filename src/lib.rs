extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort_call_site;
use syn;
use quote::quote;
use darling::{FromDeriveInput, FromField};

use interactive_clap::ToCli;



#[derive(Clone, Debug, FromField)]
struct QueryField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
}

#[derive(Debug, FromDeriveInput)]
// При помощи этого атрибута мы ограничиваемся поддержкой только именованных
// структур, если мы попробуем использовать наш макрос на других типах структур
// или перечислениях, то получим ошибку.
#[darling(supports(struct_named))]
struct InteractiveClap {
    ident: syn::Ident,
    // В таком вот незамысловатом виде мы получаем список полей в уже
    // разобранном виде.
    // В darling::ast::Data два шаблонных параметра: первый это поля
    // перечисления, а второй это поля структуры.
    // Так как в данный момент перечисления нас не интересуют, то мы можем
    // просто указать ().
    data: darling::ast::Data<(), QueryField>,
}

#[proc_macro_derive(InteractiveClap)]
pub fn interactive_clap_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse_macro_input!(input);

    // Build the trait implementation
    impl_interactive_clap_derive(&ast)
}

fn impl_interactive_clap_derive(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let cli_name_string = format!("Cli{}", &ast.ident);
    let cli_name = &syn::Ident::new(&cli_name_string, Span::call_site());

    match &ast.data {
        syn::Data::Struct(_) => {
            // darling
            let interactive_clap_derive = match InteractiveClap::from_derive_input(&ast) {
                Ok(parsed) => parsed,
                Err(e) => return e.write_errors().into(),
            };
            let fields = interactive_clap_derive.data.clone().take_struct().unwrap();
            let cli_fields = fields.iter().map(|field| {
                let ident = &field.ident;
                let ty = &field.ty;
                quote! {
                    #ident: Option<<#ty as ToCli>::CliVariant>
                }
            });
            let from_cli_fields = fields.iter().map(|field| {
                let ident = &field.ident.clone().unwrap();
                let ty = &field.ty;
                
                let fn_input_string = format!("input_{}", &ident);
                let fn_input = &syn::Ident::new(&fn_input_string, Span::call_site());
                quote! {
                    #ident: match item.#ident {
                        Some(cli_input) => #ty::from(cli_input),
                        None => #name::#fn_input()
                    }
                }
            });
            // classic
            let gen = quote! {
                impl std::fmt::Display for #name {
                    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(fmt, "Hello, Macro! My name is {}!", stringify!(#name))
                    }
                }
                #[derive(Debug, Default, clap::Clap)]
                struct #cli_name {
                    #( #cli_fields, )*
                }
                
                impl From<#cli_name> for #name {
                    fn from(item: #cli_name) -> Self {
                        Self {
                            #( #from_cli_fields, )*
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Enum(syn::DataEnum { variants, .. }) => {
            let enum_variants = variants.iter().map(|variant| {
                let ident = &variant.ident;
                let fields = &variant.fields;
                match fields {
                    syn::Fields::Named(_field) => {
                        quote! { #ident }
                    },
                    syn::Fields::Unnamed(field) => {
                        let query_field = QueryField::from_field(&field.unnamed[0]).unwrap();
                        let ty = query_field.ty;
                        quote! { #ident(#ty) }
                        
                    },
                    syn::Fields::Unit => quote! { #ident },
                }
            });
            let gen = quote! {
                #[derive(Debug, clap::Clap)]
                enum #cli_name {
                    #( #enum_variants, )*
                }
            };
            gen.into()
        }
        _ => abort_call_site!("`#[derive(InteractiveClap)]` only supports structs and enums")
    }
}
