mod field;
mod schema;
mod encode;
mod decode;
mod utils;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(Encode, attributes(schema))]
pub fn encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    encode::encode(input).into()
}

#[proc_macro_derive(Decode, attributes(schema))]
pub fn decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    decode::decode(input).into()
}
