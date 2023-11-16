use std::str::FromStr;
extern crate shared_lib;
use proc_macro::{TokenStream, Span};
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro]
pub fn css(tokens: TokenStream) -> TokenStream {

    let ss =  rcss::parse2(tokens.into()).unwrap();
    let tokens_str = Span::call_site().source_text().unwrap();
    proc_macro2::fallback::force();
    // dbg!(&rcss::parse2(tokens.into()));

    
    // let res = shared_lib::the_macro(tokens_str.clone());

    let res = rcss::parse2_with_macro_call(TokenStream2::from_str(&tokens_str).unwrap()).ok();
    dbg!(&res);
    assert_eq!(res.unwrap().to_string(), ss.to_string());

    // let stream =  rcss::parse2(TokenStream2::from_str(&tokens_str).unwrap());
    // panic!("{:?}", stream);

    // panic!("{:?}", TokenStream2::from_str(&tokens_str).unwrap());
    todo!{}
}