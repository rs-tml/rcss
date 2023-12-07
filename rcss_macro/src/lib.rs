use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use quote::quote;

use rcss_core::{self, macro_helper::macro_input, CssEmbeding, CssOutput};

// #[cfg(all(feature = "file", feature = "inline"))]
// compile_error!("Can't use both file and inline features at the same time");
// #[cfg(all(feature = "scoped", feature = "css_module"))]
// compile_error!("Can't use both scoped and css_module features at the same time");

#[cfg(any(
    all(feature = "lightningcss", feature = "postcss"),
    all(feature = "lightningcss", feature = "stylers"),
    all(feature = "postcss", feature = "stylers")
))]
compile_error!("Can't use more than one css processor at the same time");

mod fallback_ide;

#[proc_macro]
pub fn css_module(tokens: TokenStream) -> TokenStream {
    let v = css_inner(false, CssEmbeding::CssModules)
        .unwrap_or_else(|| fallback_ide::parse(tokens.into()));
    return v.generate_css_module(None).into();
}
#[proc_macro]
pub fn css_module_inline(tokens: TokenStream) -> TokenStream {
    let v = css_inner(false, CssEmbeding::CssModules)
        .unwrap_or_else(|| fallback_ide::parse(tokens.into()));

    let module = v.generate_css_module(None);
    let style = v.to_string();

    let is_proc_macro = proc_macro::is_available();
    let envs = std::env::vars()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    let debug = format!("is_proc_macro = {}, envs = {}", is_proc_macro, envs);
    quote! {
        ({let _ = #debug;
            #module
        },
        #style)
    }
    .into()
}

#[proc_macro]
pub fn css_scoped(tokens: TokenStream) -> TokenStream {
    let v =
        css_inner(false, CssEmbeding::Scoped).unwrap_or_else(|| fallback_ide::parse(tokens.into()));
    let class_name = v.class_name();
    return quote! {

        #class_name

    }
    .into();
}
#[proc_macro]
pub fn css_scoped_inline(_tokens: TokenStream) -> TokenStream {
    if let Some(v) = css_inner(false, CssEmbeding::Scoped) {
        let class_name = v.class_name();
        let style = v.to_string();

        return quote! {

            (
                 #class_name,
            #style
            )

        }
        .into();
    }
    quote! {}.into()
}

/// Generate struct with css classes as fields.
/// Uses ident from first macro argument as struct name.
///
/// Example:
/// ```rust
/// css_struct! {
///  Foo =>
///   .my-class {
///    color: red;
///  }
/// };
/// let foo = Foo::new();
/// assert!(foo.my_class.contains("my-class"));
/// ```
#[proc_macro]
// #[cfg(feature = "css_module")]
pub fn css_module_struct(tokens: TokenStream) -> TokenStream {
    let tokens: proc_macro2::TokenStream = tokens.into();
    let mut token_iter = tokens.into_iter();
    let Some(TokenTree::Ident(ident)) = token_iter.next() else {
        return quote! {
            compile_error!("Expected struct name")
        }
        .into();
    };
    let eq = token_iter.next(); // =
    let gt = token_iter.next(); // >
    if matches!(eq, Some(TokenTree::Punct(p)) if p.as_char() == '=') {
        return quote! {
            compile_error!("Expected =>")
        }
        .into();
    }
    if matches!(gt, Some(TokenTree::Punct(p)) if p.as_char() == '>') {
        return quote! {
            compile_error!("Expected =>")
        }
        .into();
    }

    let v = css_inner(true, CssEmbeding::CssModules)
        .unwrap_or_else(|| fallback_ide::parse(token_iter.collect()));
    return v.generate_css_module(Some(ident)).into();
}

#[proc_macro]
pub fn css_module_struct_inline(tokens: TokenStream) -> TokenStream {
    let tokens: proc_macro2::TokenStream = tokens.into();
    let mut token_iter = tokens.into_iter();
    let Some(TokenTree::Ident(ident)) = token_iter.next() else {
        return quote! {
            compile_error!("Expected struct name")
        }
        .into();
    };
    let comma = token_iter.next(); // ,
    if !matches!(comma, Some(TokenTree::Punct(p)) if p.as_char() == ',') {
        return quote! {
            compile_error!("Expected ,")
        }
        .into();
    }
    let Some(TokenTree::Ident(style_ident)) = token_iter.next() else {
        return quote! {
            compile_error!("Expected style variable name")
        }
        .into();
    };

    let eq = token_iter.next(); // =
    let gt = token_iter.next(); // >
    if !matches!(eq, Some(TokenTree::Punct(p)) if p.as_char() == '=') {
        return quote! {
            compile_error!("Expected =>")
        }
        .into();
    }
    if !matches!(gt, Some(TokenTree::Punct(p)) if p.as_char() == '>') {
        return quote! {
            compile_error!("Expected =>")
        }
        .into();
    }

    let v = css_inner(true, CssEmbeding::CssModules)
        .unwrap_or_else(|| fallback_ide::parse(token_iter.collect()));
    let struct_generated = v.generate_css_module(Some(ident.into()));
    let style = v.to_string();
    quote! {
        #struct_generated;

        static #style_ident: &'static str = #style;
    }
    .into()
}

/// Return None if macro input is invalid.
/// Or if source_text is not found.
fn css_inner(struct_ident_expected: bool, embeding: CssEmbeding) -> Option<CssOutput> {
    let text = Span::call_site().source_text()?;
    let text = macro_input(&text, struct_ident_expected)?;
    let mut processor =
        rcss_core::CssProcessor::new(rcss_core::CssPreprocessor::LightningCss, embeding);
    let output = processor.process_style(&text);
    return Some(output);
}
