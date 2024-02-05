use proc_macro::TokenStream;
use proc_macro2::Span;

use quote::quote_spanned;
use rcss_core::{self, CssOutput};

mod fallback_ide;
mod helpers;
use helpers::CssOutputGenerateExt;
// #[proc_macro]
// pub fn css_module(tokens: TokenStream) -> TokenStream {
//     let v = css_inner(false).unwrap_or_else(|| fallback_ide::parse(tokens.into()));
//     v.generate_css_module(None).into()
// }
// #[proc_macro]
// pub fn css_module_inline(tokens: TokenStream) -> TokenStream {
//     let v = css_inner(false).unwrap_or_else(|| fallback_ide::parse(tokens.into()));

//     let module = v.generate_css_module(None);
//     let style = v.style_string();

//     let is_proc_macro = proc_macro::is_available();
//     let envs = std::env::vars()
//         .map(|(k, v)| format!("{}={}", k, v))
//         .collect::<Vec<_>>()
//         .join("\n");
//     let debug = format!("is_proc_macro = {}, envs = {}", is_proc_macro, envs);
//     quote! {
//         ({let _ = #debug;
//             #module
//         },
//         #style)
//     }
//     .into()
// }

// #[proc_macro]
// pub fn css_scoped(tokens: TokenStream) -> TokenStream {
//     let v = css_inner(false).unwrap_or_else(|| fallback_ide::parse(tokens.into()));
//     let class_name = v.class_name();
//     quote! {

//         #class_name

//     }
//     .into()
// }
// #[proc_macro]
// pub fn css_scoped_inline(_tokens: TokenStream) -> TokenStream {
//     if let Some(v) = css_inner(false) {
//         let class_name = v.class_name();
//         let style = v.style_string();

//         return quote! {

//             (
//                 #class_name,
//                 #style
//             )

//         }
//         .into();
//     }
//     quote! {}.into()
// }

// /// Generate struct with css classes as fields.
// /// Uses ident from first macro argument as struct name.
// ///
// /// Example:
// /// ```rust
// /// rcss_macro::css_module_struct! {
// ///  Foo =>
// ///   .my-class {
// ///    color: red;
// ///  }
// /// };
// /// let foo = Foo::new();
// /// assert!(foo.my_class.contains("my-class"));
// /// ```
// #[proc_macro]
// // #[cfg(feature = "css_module")]
// pub fn css_module_struct(tokens: TokenStream) -> TokenStream {
//     let tokens: proc_macro2::TokenStream = tokens.into();
//     let mut token_iter = tokens.into_iter();
//     let Some(TokenTree::Ident(ident)) = token_iter.next() else {
//         return quote! {
//             compile_error!("Expected struct name")
//         }
//         .into();
//     };
//     let eq = token_iter.next(); // =
//     let gt = token_iter.next(); // >
//     if !matches!(eq, Some(TokenTree::Punct(p)) if p.as_char() == '=') {
//         return quote! {
//             compile_error!("Expected =>")
//         }
//         .into();
//     }
//     if !matches!(gt, Some(TokenTree::Punct(p)) if p.as_char() == '>') {
//         return quote! {
//             compile_error!("Expected =>")
//         }
//         .into();
//     }

//     let v = css_inner(true).unwrap_or_else(|| fallback_ide::parse(token_iter.collect()));
//     v.generate_css_module(Some(ident)).into()
// }

// #[proc_macro]
// pub fn css_module_mod(tokens: TokenStream) -> TokenStream {
//     let tokens: proc_macro2::TokenStream = tokens.into();
//     let mut token_iter = tokens.into_iter();
//     let ident = token_iter.next();
//     let Some(TokenTree::Ident(mod_name)) = ident else {
//         return quote_spanned! {
//             ident.span() =>
//             compile_error!("Expected struct name")
//         }
//         .into();
//     };

//     let eq = token_iter.next(); // =
//     let gt = token_iter.next(); // >
//     if !matches!(&eq, Some(TokenTree::Punct(p)) if p.as_char() == '=') {
//         return quote_spanned! {
//             eq.span()=>
//             compile_error!("Expected =>")
//         }
//         .into();
//     }
//     if !matches!(&gt, Some(TokenTree::Punct(p)) if p.as_char() == '>') {
//         return quote_spanned! {
//             gt.span() =>
//             compile_error!("Expected =>")
//         }
//         .into();
//     }

//     let v = css_inner(true).unwrap_or_else(|| fallback_ide::parse(token_iter.collect()));
//     let struct_generated = v.generate_css_module(Some(format_ident!("Css")));
//     let style = v.style_string();
//     let stream = quote! {
//         mod #mod_name {
//             #struct_generated

//             static STYLE: &'static str = #style;
//         }
//     };
//     stream.into()
// }

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
    let mut output = rcss_core::CssProcessor::process_style(&text)?;
    if cfg!(disable_styles) {
        output.clear_styles();
        // panic!("Styles was disabled.")
    }
    Ok(output)
}
