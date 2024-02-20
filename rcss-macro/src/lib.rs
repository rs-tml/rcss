use proc_macro::TokenStream;
use proc_macro2::Span;

use quote::quote_spanned;
use rcss_core::CssOutput;

mod fallback_ide;
mod helpers;
use helpers::CssOutputGenerateExt;

/// Generate CSS scope object based on css.
/// Don't use this macro directly, use rcss crate instead, since bundler will ignore macro that used directly.
#[proc_macro]
pub fn css(tokens: TokenStream) -> TokenStream {
    let output = if Span::call_site().source_text().is_some() {
        match css_inner() {
            Ok(output) => output,
            Err(e) => {
                let msg = format!("{}", e);
                return quote_spanned! {Span::call_site()=>
                    compile_error!(#msg)
                }
                .into();
            }
        }
    } else {
        fallback_ide::parse(tokens.into())
    };

    output.generate().into()
}
#[derive(thiserror::Error, Debug)]
enum MacroError {
    #[error("Failed to parse css: {0}")]
    ParseError(#[from] rcss_core::Error),
    #[error("No valid source code available for this macro call.")]
    NoSourceAvailable,
}

/// Return None if macro input is invalid.
/// Or if source_text is not found.
fn css_inner() -> Result<CssOutput, MacroError> {
    let Some(text) = Span::call_site().source_text() else {
        return Err(MacroError::NoSourceAvailable);
    };
    let Some(text) = helpers::macro_input(&text) else {
        return Err(MacroError::NoSourceAvailable);
    };
    let (interpolate, result) = rcss_core::interpolate::handle_interpolate(&text);
    let text = interpolate.unwrap_literals(result.as_ref());
    let mut output = rcss_core::CssProcessor::process_style(&text)?;
    if cfg!(disable_styles) {
        output.clear_styles();
        // panic!("Styles was disabled.")
    }
    Ok(output)
}
