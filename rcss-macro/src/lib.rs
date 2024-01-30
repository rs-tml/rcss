use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use quote::{format_ident, quote, quote_spanned};

use rcss_core::{self, macro_helper::macro_input, CssOutput};
use syn::spanned::Spanned;

mod fallback_ide;

#[proc_macro]
pub fn css_module(tokens: TokenStream) -> TokenStream {
    let v = css_inner(false).unwrap_or_else(|| fallback_ide::parse(tokens.into()));
    v.generate_css_module(None).into()
}
#[proc_macro]
pub fn css_module_inline(tokens: TokenStream) -> TokenStream {
    let v = css_inner(false).unwrap_or_else(|| fallback_ide::parse(tokens.into()));

    let module = v.generate_css_module(None);
    let style = v.style_string();

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
    let v = css_inner(false).unwrap_or_else(|| fallback_ide::parse(tokens.into()));
    let class_name = v.class_name();
    quote! {

        #class_name

    }
    .into()
}
#[proc_macro]
pub fn css_scoped_inline(_tokens: TokenStream) -> TokenStream {
    if let Some(v) = css_inner(false) {
        let class_name = v.class_name();
        let style = v.style_string();

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
/// rcss_macro::css_module_struct! {
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

    let v = css_inner(true).unwrap_or_else(|| fallback_ide::parse(token_iter.collect()));
    v.generate_css_module(Some(ident)).into()
}

#[proc_macro]
pub fn css_module_mod(tokens: TokenStream) -> TokenStream {
    let tokens: proc_macro2::TokenStream = tokens.into();
    let mut token_iter = tokens.into_iter();
    let ident = token_iter.next();
    let Some(TokenTree::Ident(mod_name)) = ident else {
        return quote_spanned! {
            ident.span() =>
            compile_error!("Expected struct name")
        }
        .into();
    };

    let eq = token_iter.next(); // =
    let gt = token_iter.next(); // >
    if !matches!(&eq, Some(TokenTree::Punct(p)) if p.as_char() == '=') {
        return quote_spanned! {
            eq.span()=>
            compile_error!("Expected =>")
        }
        .into();
    }
    if !matches!(&gt, Some(TokenTree::Punct(p)) if p.as_char() == '>') {
        return quote_spanned! {
            gt.span() =>
            compile_error!("Expected =>")
        }
        .into();
    }

    let v = css_inner(true).unwrap_or_else(|| fallback_ide::parse(token_iter.collect()));
    let struct_generated = v.generate_css_module(Some(format_ident!("Css")));
    let style = v.style_string();
    let stream = quote! {
        mod #mod_name {
            #struct_generated

            static STYLE: &'static str = #style;
        }
    };
    stream.into()
}

/// Return None if macro input is invalid.
/// Or if source_text is not found.
fn css_inner(struct_ident_expected: bool) -> Option<CssOutput> {
    let text = Span::call_site().source_text()?;
    let text = macro_input(&text, struct_ident_expected)?;
    let output = rcss_core::CssProcessor::process_style(&text);
    Some(output)
}
