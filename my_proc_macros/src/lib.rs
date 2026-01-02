extern crate proc_macro;
use event::parse_key_event;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum};

#[proc_macro]
pub fn key(input: TokenStream) -> TokenStream {
    let str = remove_quotes(input);

    let event = parse_key_event(&str).unwrap();

    event.to_rust_code().parse().unwrap()
}

fn remove_quotes(input: TokenStream) -> String {
    let str = input.to_string();
    str[1..str.len() - 1].to_string()
}

#[proc_macro]
pub fn keys(input: TokenStream) -> TokenStream {
    let str = remove_quotes(input);

    let events = event::parse_key_events(&str)
        .unwrap()
        .into_iter()
        .map(|event| event.to_rust_code())
        .collect::<Vec<_>>()
        .join(", ");

    format!("&[{events}]").parse().unwrap()
}

#[proc_macro]
pub fn hex(input: TokenStream) -> TokenStream {
    let hex = remove_quotes(input);
    let regex = regex::Regex::new(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$").unwrap();
    if !regex.is_match(&hex) {
        panic!("Invalid hex color: {hex}");
    }

    let hex = &hex[1..];

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();

    format!("crate::themes::Color::new({r}, {g}, {b})")
        .parse()
        .unwrap()
}

#[proc_macro_derive(NamedVariant)]
pub fn derive_named_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemEnum);
    let enum_name = &input.ident;

    let match_arms = input.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_name_str = variant_name.to_string();
        match &variant.fields {
            syn::Fields::Named(_) => {
                quote!( #enum_name::#variant_name { .. } => #variant_name_str, )
            }
            syn::Fields::Unnamed(_) => {
                quote!( #enum_name::#variant_name(..) => #variant_name_str, )
            }
            syn::Fields::Unit => quote!( #enum_name::#variant_name => #variant_name_str, ),
        }
    });

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            fn variant_name(&self) -> &'static str {
                match self {
                    #(#match_arms)*
                }
            }
        }
    })
}
