extern crate proc_macro;
use event::parse_key_event;
use proc_macro::TokenStream;

#[proc_macro]
pub fn key(item: TokenStream) -> TokenStream {
    let str = item.to_string();

    // Remove the quotes
    let str = &str[1..str.len() - 1];

    let event = parse_key_event(str).unwrap();

    event.to_rust_code().parse().unwrap()
}

#[proc_macro]
pub fn keys(item: TokenStream) -> TokenStream {
    let str = item.to_string();

    // Remove the quotes
    let str = &str[1..str.len() - 1];

    let events = event::parse_key_events(str)
        .unwrap()
        .into_iter()
        .map(|event| event.to_rust_code())
        .collect::<Vec<_>>()
        .join(", ");

    format!("&[{}]", events).parse().unwrap()
}
