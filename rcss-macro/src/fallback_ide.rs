//!
//! Fallback parser specifically for IDEs and css_modules api.
//! Collect all classes (in reallity any dot prefixed identifiers, and some dash expressions).
//!
//! Outputs CssOutput with all classes and empty css strings and uniq class name.
//!
//! Can generate false positives when property or at-rule arguments receive some dot prefixed identifiers.
use proc_macro2::{TokenStream, TokenTree};
use rcss_core::{ClassInfo, CssOutput};

pub fn parse(input: TokenStream) -> CssOutput {
    let Ok(v) = std::env::var("RUST_ANALYZER_INTERNALS_DO_NOT_USE") else {
        panic!("fallback only available for rust-analyzer, for regular source_text should be available")
    };
    if v != "this is unstable" {
        panic!("RUST_ANALYZER_INTERNALS_DO_NOT_USE is not set")
    }
    parse_inner(input)
}

fn parse_inner(input: TokenStream) -> CssOutput {
    let mut stack = vec![input];
    let mut classes = Vec::new();
    let mut declare = None;
    let mut extends = None;
    while let Some(input) = stack.pop() {
        let mut tokens = input.into_iter().peekable();
        while let Some(token) = tokens.next() {
            let punct = match token {
                TokenTree::Punct(punct) => punct,
                TokenTree::Group(group) => {
                    stack.push(group.stream());
                    continue;
                }
                TokenTree::Literal(_) | TokenTree::Ident(_) => {
                    continue;
                }
            };
            match punct.as_char() {
                '.' => {
                    // process later
                }
                '@' => {
                    // at rule
                    let Some(rcss_rule) = crate::helpers::parse_rcss_config(&mut tokens) else {
                        continue;
                    };
                    match rcss_rule {
                        rcss_core::rcss_at_rule::RcssAtRuleConfig::Struct(item_struct) => {
                            declare = Some(item_struct)
                        }
                        rcss_core::rcss_at_rule::RcssAtRuleConfig::Extend(path) => {
                            extends = Some(path)
                        }
                    }
                    continue;
                }
                _ => {
                    continue;
                }
            }
            // ignore literals after dot (prevent parsing something like .32);
            if let Some(TokenTree::Literal(_)) = tokens.peek() {
                continue;
            }

            let mut ident = Vec::new();
            'collect_ident_fragments: loop {
                match tokens.next() {
                    Some(tt @ TokenTree::Ident(_)) => {
                        ident.push(tt);

                        // Only ident followed by dash is allowed
                        if let Some(TokenTree::Punct(_)) = tokens.peek() {
                        }
                        // Ident followed by ident or literal has space between
                        else {
                            break 'collect_ident_fragments;
                        }
                    }
                    Some(TokenTree::Literal(l)) => {
                        let str_lit = l.to_string();
                        // No strings is allowed in class names
                        if str_lit.starts_with('"') || str_lit.ends_with('\'') {
                            break 'collect_ident_fragments;
                        }
                        ident.push(TokenTree::Literal(l));

                        // Only literals followed by dash is allowed
                        if let Some(TokenTree::Punct(_)) = tokens.peek() {
                        }
                        // Ident followed by ident or literal has space between
                        else {
                            break 'collect_ident_fragments;
                        }
                    }
                    Some(TokenTree::Punct(p)) => {
                        if p.as_char() == '-' {
                            ident.push(TokenTree::Punct(p));
                        } else {
                            break 'collect_ident_fragments;
                        }
                    }
                    Some(TokenTree::Group(group)) => {
                        stack.push(group.stream());
                        break 'collect_ident_fragments;
                    }
                    _ => {
                        break 'collect_ident_fragments;
                    }
                }
            }
            if !ident.is_empty() {
                classes.push(ident);
            }
        }
    }
    let output = CssOutput::create_from_fields(
        String::from(""),
        String::from(""),
        declare,
        extends,
        classes
            .into_iter()
            .map(|ident| {
                let first_span = ident.first().unwrap().span();
                let mut span = first_span;
                let ident = ident.into_iter();
                let mut ident_str = String::new();
                for tt in ident {
                    ident_str.push_str(&tt.to_string());
                    span = span.join(tt.span()).unwrap_or(span);
                }
                (
                    ident_str.clone(),
                    ClassInfo {
                        class_name: ident_str,
                        original_span: Some(span),
                    },
                )
            })
            .collect(),
    );
    output
}

#[cfg(test)]
mod test {
    use quote::ToTokens;

    // check different idents
    #[test]
    fn check_parse() {
        let input = r#"
        .my-class {
            color: red;
        }
        .class_with_dash {
            color: red;
        }
        .class_with_attribute[fake-attr] {
            color: red;
        }
        .class_with_attribute2[fake-attr="fake-value"] {
            color: red;
        }
        .multiselector, .multiselector2 {
            color: red;
        }
        .child-selector > .child-selector2 {
            color: red;
        }
        regular elements .child-selector2 regular elements {
            color: red;
        }
        
        "#;
        let output = super::parse_inner(input.parse().unwrap());
        let elements_list = output.classes_list().collect::<Vec<_>>();
        let mut expected_list = vec![
            "my-class",
            "class_with_dash",
            "class_with_attribute",
            "class_with_attribute2",
            "multiselector",
            "multiselector2",
            "child-selector",
            "child-selector2",
        ];
        expected_list.sort();
        assert_eq!(elements_list, expected_list);
    }

    #[test]
    fn check_parse_inner() {
        let input = r#"
        .my-class {
            color: red;
            & .my-class2 {
                color: red;
            }
        }
        
        
        "#;
        let output = super::parse_inner(input.parse().unwrap());
        let elements_list = output.classes_list().collect::<Vec<_>>();
        let mut expected_list = vec!["my-class", "my-class2"];
        expected_list.sort();
        assert_eq!(elements_list, expected_list);
    }

    #[test]
    fn ignore_dotted_num() {
        let input = r#"
        .my-class {
            width: .32;
        }
        "#;
        let output = super::parse_inner(input.parse().unwrap());
        let elements_list = output.classes_list().collect::<Vec<_>>();
        let expected_list = vec!["my-class"];

        assert_eq!(elements_list, expected_list);
    }

    #[test]
    fn mod_declare_and_extend_parse() {
        let input = r#"
        @rcss(mod my_mod);
        @rcss(extend ::path::to::my_mod);
        .my-class {
            color: red;
        }
        "#;
        let output = super::parse_inner(input.parse().unwrap());
        let elements_list = output.classes_list().collect::<Vec<_>>();
        let mut expected_list = vec!["my-class"];
        expected_list.sort();
        assert_eq!(elements_list, expected_list);
        assert_eq!(output.declare().unwrap().ident.to_string(), "my_mod");
        assert_eq!(
            output.extend().unwrap().to_token_stream().to_string(),
            ":: path :: to :: my_mod"
        );
    }
}
