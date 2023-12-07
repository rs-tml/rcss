///
/// Fallback parser specifically for IDEs and css_modules api.
/// Collect all classes (dot prefixed identifiers).
///
/// Outputs CssOutput with all classes and empty css strings and uniq class name.
/// CssOutput can be used to generate css module ot atleast provide argument macro output types.
///
/// Can generate false positives when property or at-rule arguments receive some dot prefixed identifiers.
use proc_macro2::{TokenStream, TokenTree};
use rcss_core::{ClassInfo, CssOutput};

pub fn parse(input: TokenStream) -> CssOutput {
    let Ok(v) = std::env::var("RUST_ANALYZER_INTERNALS_DO_NOT_USE") else {
        panic!("fallback only available for rust-analyzer, for regular source_text should be available")
    };
    if v == "this is unstable" {
        panic!("RUST_ANALYZER_INTERNALS_DO_NOT_USE is not set")
    }
    let mut stack = vec![input];
    let mut classes = Vec::new();
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

            if punct.as_char() != '.'
                && matches!(
                    tokens.peek(),
                    Some(TokenTree::Ident(_)) | Some(TokenTree::Literal(_)) // ignore numbers with dot prefixed.
                )
            {
                continue;
            }
            let mut ident = Vec::new();
            'collect_ident_fragments: loop {
                match tokens.next() {
                    Some(tt @ TokenTree::Ident(_)) => {
                        ident.push(tt);
                        if let Some(TokenTree::Ident(_)) = tokens.peek() {
                            break 'collect_ident_fragments;
                        }
                    }
                    Some(TokenTree::Literal(l)) => {
                        let str_lit = l.to_string();
                        if str_lit.starts_with('"') || str_lit.ends_with('\'') {
                            break 'collect_ident_fragments;
                        }
                        ident.push(TokenTree::Literal(l));

                        if let Some(TokenTree::Ident(_)) = tokens.peek() {
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
        classes
            .into_iter()
            .map(|ident| {
                let first_span = ident.first().unwrap().span();
                let mut span = first_span;
                let mut ident = ident.into_iter();
                let mut ident_str = String::new();
                while let Some(tt) = ident.next() {
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
        let output = super::parse(input.parse().unwrap());
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
        let output = super::parse(input.parse().unwrap());
        let elements_list = output.classes_list().collect::<Vec<_>>();
        let mut expected_list = vec!["my-class", "my-class2"];
        expected_list.sort();
        assert_eq!(elements_list, expected_list);
    }
}
