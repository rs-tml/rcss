use proc_macro2::TokenStream as TokenStream2;
use quote::spanned::Spanned;

// TODO: add ident checks and other "token_trees"
/// Get macro input from macro call source text.
/// Input should be in format: `css_module! { ... }`
/// And can be retrived in function like proc-macro `Span::call_site().source_text()`.
///
/// Example:
/// ```rust
/// let input = r#"
/// css_module! {
///    .my-class {
///       color: red;
///   }
/// }
/// "#;
/// let macro_input = rcss_core::macro_helper::macro_input(input, false).unwrap();
/// assert_eq!(macro_input, ".my-class {\n      color: red;\n  }");
/// ```
pub fn macro_input(source_text: &str, skip_ident_arrow: bool) -> Option<String> {
    // 1. Find macro call group (any type of braces)
    // 2. skip whitespaces
    // 3. if skip_ident_arrow is present, skip ident, =, > at begginning
    // 4. return rest of the string or None, if group wasn't found
    let (_path, group_start) = source_text.split_once(|c| "{[(".contains(c))?;
    let (mut group, _end) = group_start.rsplit_once(|c| "}])".contains(c))?;

    if skip_ident_arrow {
        let mut iter = group.splitn(3, |c| c == '=' || c == '>');
        iter.next(); // Foo [, variable_name]
        iter.next(); // =(empty string between tokens)>
        group = iter.next().unwrap_or("").trim();
    }
    let trimed = group.trim();

    return Some(trimed.to_owned());
}

/// Get macro input from macro call source text.
/// Using proc_macro2::TokenStream to parse input and skeep tokens.
///
/// Example:
/// ```rust
/// let input = r#"
/// css_module! {
///    .my-class {
///       color: red;
///   }
/// }
/// "#;
/// let macro_input = rcss_core::macro_helper::macro_input_with_token_stream(input, false).unwrap();
/// assert_eq!(macro_input, ".my-class {\n      color: red;\n  }");
/// ```
pub fn macro_input_with_token_stream(source_text: &str, skip_ident_arrow: bool) -> Option<String> {
    use std::str::FromStr;
    // 1. Find macro call group (any type of braces)
    // 2. skip whitespaces
    // 3. if skip_ident_arrow is present, skip ident, =, > at begginning
    // 4. return rest of the string or None, if group wasn't found
    proc_macro2::fallback::force();
    let stream = TokenStream2::from_str(source_text).unwrap();
    let mut stream_iter = stream.into_iter();

    // Skip path to the macro with exclamation mark:
    // example: "foo::module::bar! /*stop */ {...}"
    while let Some(tt) = stream_iter.next() {
        if let proc_macro2::TokenTree::Punct(p) = tt {
            if p.as_char() == '!' {
                break;
            }
        }
    }

    if let Some(proc_macro2::TokenTree::Group(g)) = stream_iter.next() {
        let mut stream_iter = g.stream().into_iter();
        if skip_ident_arrow {
            while let Some(tt) = stream_iter.next() {
                let proc_macro2::TokenTree::Punct(p) = tt else {
                    continue;
                };
                if p.as_char() == '=' {
                    // =
                    break;
                }
            }
            stream_iter.next(); // >
        }

        let source = stream_iter.collect::<TokenStream2>();

        source.__span().source_text()
    } else {
        None
    }
}
#[cfg(test)]
mod test {
    #[test]
    fn check_macro_input_extractor() {
        let input = r#"
        css_module! {
            .my-class {
                color: red;
            }
        }
        "#;
        let macro_input = super::macro_input_with_token_stream(input, false).unwrap();
        let compare_optimized = super::macro_input(input, false).unwrap();
        assert_eq!(
            macro_input,
            ".my-class {\n                color: red;\n            }"
        );
        assert_eq!(macro_input, compare_optimized);
    }

    #[test]
    fn check_macro_input_extractor_struct() {
        let input = r#"
        css_struct! {
            Foo =>
            .my-class {
                color: red;
            }
        }
        "#;
        let macro_input = super::macro_input_with_token_stream(input, true).unwrap();
        let compare_optimized = super::macro_input(input, true).unwrap();
        assert_eq!(
            macro_input,
            ".my-class {\n                color: red;\n            }"
        );
        assert_eq!(macro_input, compare_optimized);
    }
}
