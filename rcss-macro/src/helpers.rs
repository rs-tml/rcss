use std::{collections::BTreeMap, iter::Peekable};

use proc_macro2::{Literal, TokenTree};

use rcss_core::rcss_at_rule::RcssAtRuleConfig;

// TODO: add ident checks and other "token_trees"
/// Get macro input from macro call source text.
/// Input should be in format: `css! { ... }`
/// And can be retrived in function like proc-macro `Span::call_site().source_text()`.
///
/// Example:
/// ```no_build
/// let input = r#"
/// css! {
///    .my-class {
///       color: red;
///   }
/// }
/// "#;
/// let macro_input = rcss_macro::helpers::macro_input(input, false).unwrap();
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

#[derive(PartialEq, PartialOrd, Ord, Eq, Debug)]
enum Key {
    Valid(String),
    Replaced { original: String, replaced: String },
}
impl Key {
    fn field_str(&self) -> &str {
        match self {
            Key::Valid(s) => &*s,
            Key::Replaced { replaced, .. } => &*replaced,
        }
    }
    fn original(&self) -> &str {
        match self {
            Key::Valid(s) => &*s,
            Key::Replaced { original, .. } => &*original,
        }
    }
}

impl CssOutputGenerateExt for rcss_core::CssOutput {
    fn generate(&self) -> proc_macro2::TokenStream {
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

        let changed_classes: BTreeMap<_, _> = changed_classes
            .into_iter()
            .map(|(k, v)| {
                if !is_valid_rust_ident(&k) {
                    let new_key = format!("__kebab__{}", k.replace("-", "_k_"));
                    assert!(is_valid_rust_ident(&new_key));
                    (
                        Key::Replaced {
                            original: k,
                            replaced: new_key,
                        },
                        v,
                    )
                } else {
                    (Key::Valid(k), v)
                }
            })
            .collect();

        let struct_ident = self
            .declare()
            .map(|s| s.ident)
            .unwrap_or(syn::parse_quote! { Css });

        let index_match_fields = changed_classes.iter().filter_map(|(k, _v)| {
            let val: proc_macro2::Ident = quote::format_ident!("{}", k.field_str());
            let idx = k.original();
            Some(quote::quote! {
                #idx => self.#val,
            })
        });

        let uniq_class = self.class_name();

        let style: String = self.style_string();

        let vis_struct = self
            .declare()
            .map(|s| {
                let vis = &s.vis;
                let struct_ = &s.struct_token;
                quote::quote!(#vis #struct_)
            })
            .unwrap_or(quote::quote! { pub struct });

        // TODO: find a way to warn on generated dead code (fields that wasn't accessed).
        let mut struct_impl = if let Some(extend) = self.extend() {
            let root_field_init = changed_classes.iter().map(|(k, v)| {
                let span = v.original_span.unwrap_or(proc_macro2::Span::call_site());
                let k = quote::format_ident!("{}", k.field_str(), span = span);
                let v = Literal::string(&v.class_name);
                quote::quote! {
                    root.#k = ::rcss::reexport::const_format::concatcp!(ROOT.#k, " ", #v)
                }
            });
            generate_child_struct(
                vis_struct,
                &struct_ident,
                extend,
                &style,
                &uniq_class,
                root_field_init,
            )
        } else {
            let field_init_struct = changed_classes.iter().map(|(k, v)| {
                let span = v.original_span.unwrap_or(proc_macro2::Span::call_site());
                let k = quote::format_ident!("{}", k.field_str(), span = span);
                let v = Literal::string(&v.class_name);
                quote::quote! {
                    #k: #v
                }
            });
            let field_classes = changed_classes.iter().map(|(k, _v)| {
                let field = quote::format_ident!("{}", k.field_str());
                let pub_ = if matches!(k, Key::Valid(_)) {
                    quote::quote! { pub }
                } else {
                    quote::quote! { #[doc(hidden)] pub}
                };
                quote::quote! {
                    #pub_ #field
                }
            });
            generate_root_struct(
                vis_struct,
                &struct_ident,
                &style,
                &uniq_class,
                index_match_fields,
                field_classes,
                field_init_struct,
            )
        };
        // Convert to expression if it's not a declaration
        if self.declare().is_none() {
            struct_impl = quote::quote! {
                {
                #struct_impl
                   #struct_ident::new()
                }
            };
        };

        struct_impl
    }
}

fn generate_root_struct(
    vis_struct: proc_macro2::TokenStream,
    struct_ident: &proc_macro2::Ident,
    style: &str,
    uniq_class: &str,
    index_match_fields: impl Iterator<Item = proc_macro2::TokenStream>,
    field_classes: impl Iterator<Item = proc_macro2::TokenStream>,
    field_init: impl Iterator<Item = proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let index_impl = if cfg!(feature = "indexed-classes") {
        quote::quote! {
            impl<'a> std::ops::Index<&'a str> for #struct_ident {
                type Output = str;
                fn index(&self, index: &'a str) -> &Self::Output {
                    match index {
                        #(#index_match_fields)*
                        _ => panic!("Has no such key"),
                    }
                }
            }
        }
    } else {
        quote::quote! {}
    };

    quote::quote! {
        // allow non_snake_case for `__kebab__baz_k_2` style fields
        #[allow(non_snake_case)]
        #[must_use = "Scope style should be registered"]
        #[derive(Debug, Copy, Clone)]
        #vis_struct #struct_ident {
            #(#field_classes: &'static str),*
        }
        impl #struct_ident {

            pub fn new() -> Self {
                Self::new_root()
            }

            /// Const fn is not stabilized in traits, so we use it in structure.
            /// Methods new_root is used to create constant time root object, and modify thier content.
            /// It allow us to have resulted css object on compile time.
            ///
            ///
            /// TODO: Later when const trait will be stabilized we can move it into ScopeChain trait.
            pub const fn new_root() -> Self {
                Self {
                    #(#field_init),*
                }
            }


        }
        impl Default for #struct_ident {
            fn default() -> Self {
                Self::new()
            }
        }


        #[must_use = "Scope style should be registered"]
        impl ::rcss::ScopeCommon for #struct_ident {
            const STYLE: &'static str = #style;
            const SCOPE_ID: &'static str = #uniq_class;
        }

        impl ::rcss::extend::ScopeChain for #struct_ident {
            type Parent = ::std::convert::Infallible;
            type Root = Self;
        }
        #index_impl
    }
}

fn generate_child_struct(
    vis_struct: proc_macro2::TokenStream,
    struct_ident: &proc_macro2::Ident,
    path_to_parent: syn::Path,
    style: &str,
    uniq_class: &str,
    field_init: impl Iterator<Item = proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    quote::quote! {



        #[derive(Debug, Copy, Clone)]
        #vis_struct #struct_ident (#path_to_parent);
        impl #struct_ident {

            pub fn new() -> Self {
                let root = Self::new_root();
                root.into()
            }

            /// Const fn is not stabilized in traits, so we use it in structure.
            /// Methods new_root is used to create constant time root object, and modify thier content.
            /// It allow us to have resulted css object on compile time.
            ///
            ///
            /// TODO: Later when const trait will be stabilized we can move it into ScopeChain trait.
            pub const fn new_root() -> <Self as ::rcss::extend::ScopeChain>::Root {
                const ROOT: <#struct_ident as ::rcss::extend::ScopeChain>::Root = <#struct_ident as ::rcss::extend::ScopeChain>::Root::new_root();
                let mut root = ROOT;
                #(#field_init;)*
                root
            }

        }


        impl std::ops::Deref for #struct_ident {
            type Target = #path_to_parent;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl std::ops::DerefMut for #struct_ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Default for #struct_ident {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::rcss::ScopeCommon for #struct_ident {
            const STYLE: &'static str = #style;
            const SCOPE_ID: &'static str = #uniq_class;
        }


        impl ::rcss::extend::ScopeChain for #struct_ident {
            type Parent = #path_to_parent;
            type Root = <Self::Parent as ::rcss::extend::ScopeChain>::Root;
        }

        impl From<#struct_ident> for <#struct_ident as ::rcss::extend::ScopeChain>::Root {
            fn from(v: #struct_ident) -> Self {
                v.0.into()
            }
        }

        impl From<<#struct_ident as ::rcss::extend::ScopeChain>::Root> for #struct_ident {
            fn from(v: <#struct_ident as ::rcss::extend::ScopeChain>::Root) -> Self {
                Self(v.into())
            }
        }
    }
}

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
