use std::iter::Peekable;

use proc_macro2::{Literal, TokenTree};

use rcss_core::rcss_at_rule::RcssAtRuleConfig;

// TODO: add ident checks and other "token_trees"
/// Get macro input from macro call source text.
/// Input should be in format: `css! { ... }`
/// And can be retrived in function like proc-macro `Span::call_site().source_text()`.
///
/// Example:
/// ```rust
/// let input = r#"
/// css! {
///    .my-class {
///       color: red;
///   }
/// }
/// "#;
/// let macro_input = rcss_core::macro_helper::macro_input(input, false).unwrap();
/// assert_eq!(macro_input, ".my-class {\n      color: red;\n  }");
/// ```
pub fn macro_input(source_text: &str) -> Option<String> {
    // 1. Find macro call group (any type of braces)
    // 2. skip whitespaces
    // 3. return rest of the string or None, if group wasn't found
    let (_path, group_start) = source_text.split_once(|c| "{[(".contains(c))?;
    let (group, _end) = group_start.rsplit_once(|c| "}])".contains(c))?;

    let trimed = group.trim();

    Some(trimed.to_owned())
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
        let compare_optimized = super::macro_input(input).unwrap();
        assert_eq!(
            compare_optimized,
            ".my-class {\n                color: red;\n            }"
        );
    }
}

/// Parses rcssAtRule from iterator of TokenTree.
/// Expects that caller already take @ symbol.
pub fn parse_rcss_config<I>(tokens: &mut Peekable<I>) -> Option<RcssAtRuleConfig>
where
    I: Iterator<Item = TokenTree>,
{
    if matches!(tokens.peek(), Some(TokenTree::Ident(i)) if i.to_string() == "rcss" ) {
        let _rcss = tokens.next().unwrap();
        if let Some(TokenTree::Group(group)) = tokens.next() {
            if let Ok(rcss_rule) = RcssAtRuleConfig::from_token_stream(group.stream()) {
                return Some(rcss_rule);
            }
        } else {
            println!("Expected group in parsing rcss at rule");
        }
    }
    None
}

pub trait CssOutputGenerateExt {
    fn generate(&self) -> proc_macro2::TokenStream;
}

impl CssOutputGenerateExt for rcss_core::CssOutput {
    fn generate(&self) -> proc_macro2::TokenStream {
        fn is_valid_rust_ident(ident: &str) -> bool {
            let mut chars = ident.chars();
            match chars.next() {
                Some(c) if c.is_alphabetic() || c == '_' => {}
                _ => return false,
            }
            for c in chars {
                if !c.is_alphanumeric() && c != '_' {
                    return false;
                }
            }
            true
        }

        // dbg!(&self);
        #[allow(unused_mut)] //used in feature
        let mut changed_classes = self.classes_map().clone();
        #[cfg(feature = "auto-snake-case")]
        {
            use inflector::cases::snakecase::to_snake_case;
            for (k, v) in self.classes_map() {
                if !is_valid_rust_ident(k) {
                    let mut snake_case = to_snake_case(k);
                    if snake_case.chars().next().unwrap().is_numeric() {
                        snake_case = format!("_{}", snake_case);
                    }
                    changed_classes.insert(snake_case, v.clone());
                }
            }
        }

        let field_classes = changed_classes.iter().filter_map(|(k, _v)| {
            if is_valid_rust_ident(k) {
                Some(quote::format_ident!("{}", k))
            } else {
                None
            }
        });

        let field_classes_literals_match = changed_classes.iter().filter_map(|(k, _v)| {
            if is_valid_rust_ident(k) {
                let val: proc_macro2::Ident = quote::format_ident!("{}", k);
                Some(quote::quote! {
                    #k => self.#val,
                })
            } else {
                None
            }
        });

        let field_init = changed_classes
            .iter()
            .filter(|(k, _)| is_valid_rust_ident(k))
            .map(|(k, v)| {
                let span = v.original_span.unwrap_or(proc_macro2::Span::call_site());
                let k = quote::format_ident!("{}", k, span = span);
                let v = Literal::string(&v.class_name);
                quote::quote! {
                    #k: #v,
                }
            });

        let kebab_map_init = changed_classes
            .iter()
            .filter(|(k, _)| !is_valid_rust_ident(k))
            .map(|(k, v)| {
                let k = Literal::string(k);
                let v = Literal::string(&v.class_name);
                quote::quote! {
                    map.insert(#k, #v);
                }
            });

        // TODO: Add rcss-atrule for this
        let vis = self
            .declare()
            .map(|s| s.vis)
            .unwrap_or(syn::parse_quote! { pub });

        let _struct = self
            .declare()
            .map(|s| s.struct_token)
            .unwrap_or(syn::parse_quote! { struct });
        let struct_ident = self
            .declare()
            .map(|s| s.ident)
            .unwrap_or(syn::parse_quote! { Css });
        let struct_ident = quote::format_ident!("Css");
        let index_impl = if cfg!(feature = "indexed-classes") {
            quote::quote! {
                impl<'a> std::ops::Index<&'a str> for #struct_ident {
                    type Output = str;
                    fn index(&self, index: &'a str) -> &Self::Output {
                        match index {
                            #(#field_classes_literals_match)*
                            other => self
                                .__kebab_styled
                                .get(other)
                                .expect(&format!("No class with name {} found in css module", other)),
                        }
                    }
                }
            }
        } else {
            quote::quote! {}
        };
        let uniq_class = self.class_name();

        let style: String = self.style_string();
        // TODO: find a way to warn on generated dead code (fields that wasn't accessed).
        let mut struct_impl = quote::quote! {

                #vis #_struct #struct_ident {
                    #(pub #field_classes: &'static str,)*
                    __scoped_class: &'static str,
                    __kebab_styled: std::collections::BTreeMap<&'static str, &'static str>,
                }
                impl #struct_ident {
                    pub fn new() -> Self {
                        let mut map = std::collections::BTreeMap::new();
                        #(#kebab_map_init)*
                        Self {
                            __kebab_styled:map,
                            ..Self::new_without_dashed_idents()
                        }
                    }

                    pub const fn new_without_dashed_idents() -> Self {
                        Self {
                            #(#field_init)*
                            __scoped_class: #uniq_class,
                            __kebab_styled: std::collections::BTreeMap::new(),
                        }
                    }

                    pub fn scoped_class(&self) -> &'static str {
                        self.__scoped_class
                    }
                    #[doc(hidden)]
                    pub fn dashed_map(&self) -> &std::collections::BTreeMap<&'static str, &'static str> {
                        &self.__kebab_styled
                    }
                    /// Creates new css module.
                    /// For classes that is valid rust ident, it will use default class name.
                    /// Extend rest with provided map.
                    pub fn with_extension(scoped: &'static str, map: std::collections::BTreeMap<&'static str, &'static str>) -> Self {
                        let mut new = Self::new();
                        new.__scoped_class = scoped;
                        std::iter::Extend::extend(&mut new.__kebab_styled, map);
                        new
                    }

                }
                impl ::rcss::CssCommon for #struct_ident {
                    const BASIC_STYLE: &'static str = #style;
                    const BASIC_SCOPE: &'static str = #uniq_class;
                    fn basic() -> #struct_ident
                    where Self:Sized
                    {
                        #struct_ident::new()
                    }
                    fn scoped_class(&self) -> &'static str {
                        self.scoped_class()
                    }
                }
                #index_impl
        };

        // Convert to expression if it's not a declaration
        if self.declare().is_none() {
            struct_impl = quote::quote! {
                {
                #struct_impl
                    ::rcss::CssWithStyle::new(#struct_ident::new(), #style)
                }
            };
        };

        struct_impl
    }
}
