use std::str::FromStr;
use proc_macro2::TokenStream;
use rcss::StyleSheet;

#[no_mangle]
pub extern "C" fn the_macro(input: String) -> Option<StyleSheet> {
    proc_macro2::fallback::force();
    let tokens = TokenStream::from_str(&input).ok()?;
    rcss::parse2_with_macro_call(tokens).ok()
}