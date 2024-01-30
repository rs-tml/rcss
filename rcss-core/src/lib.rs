// TODO:
// - [ ] Add support for css interpolation (usefull for theming, can be used with css custom properties).
// - [ ] Procss preprocessor.
// - [ ] :deep pseudo-elemenet support

use std::{collections::BTreeMap, io::Write, path::Path};

use lightningcss::{
    stylesheet::{ParserOptions, PrinterOptions},
    visitor::Visit,
};
use proc_macro2::{Ident, Literal};
use rand::{distributions::Distribution, seq::SliceRandom, Rng, SeedableRng};

/// Helper module for writing proc-macros.
#[cfg(feature = "macro-helper")]
pub mod macro_helper;

pub mod lightning_css;

#[derive(Debug)]
pub struct CssProcessor<'i> {
    style: lightningcss::stylesheet::StyleSheet<'i, 'i>,
    // use array instead of string to avoid heap allocation.
    random_ident: [char; 7],
}
impl<'src> CssProcessor<'src> {
    // TODO: Handle error
    fn new(style: &'src str) -> Self {
        let this = Self {
            random_ident: Self::init_random_class(style),
            style: lightningcss::stylesheet::StyleSheet::parse(style, ParserOptions::default())
                .unwrap(),
        };
        this
    }
    pub fn process_style(style: &'src str) -> CssOutput {
        let mut this = Self::new(style);
        this.process_style_inner()
    }

    fn process_style_inner<'a>(&mut self) -> CssOutput {
        // Create visitor that will modify class names, but will not modify css rules.
        let suffix = self.get_class_suffix();
        let mut visitor = lightning_css::SelectorVisitor {
            append_class: self.get_scoped_class(),
            class_modify: Box::new(move |class| format!("{class}-{suffix}")),
            collect_classes: BTreeMap::new(),
            state: Default::default(),
        };
        self.style.visit(&mut visitor).unwrap();
        let changed_classes = visitor
            .collect_classes
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    ClassInfo {
                        class_name: v,
                        original_span: None,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        CssOutput {
            uniq_class: visitor.append_class,
            css_data: self
                .style
                .to_css(PrinterOptions {
                    minify: true,
                    ..Default::default()
                })
                .unwrap()
                .code,
            changed_classes,
        }
    }
    fn init_random_class(style: &str) -> [char; 7] {
        struct CssIdentChars;
        impl Distribution<char> for CssIdentChars {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
                const ALLOWED_CHARS: &str =
                    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let chars: Vec<char> = ALLOWED_CHARS.chars().collect();
                *chars.choose(rng).unwrap()
            }
        }

        let mut seed = [0xdeu8; 32];
        style
            .bytes()
            .enumerate()
            .for_each(|(i, c)| seed[i % 32] ^= c);

        let rng = rand_chacha::ChaCha8Rng::from_seed(seed);

        let ident_vec = std::iter::once('_')
            .chain(rng.sample_iter(CssIdentChars).take(6))
            .collect::<Vec<_>>();
        std::array::from_fn(|i| ident_vec[i])
    }
    // Returns random class identifier.
    // Each call return same result for same preprocessor input.
    // Return 7 symbol with first symbol as underscore.
    fn get_scoped_class(&self) -> String {
        self.random_ident.iter().collect::<String>()
    }

    // Returns random class suffix.
    // Each call return same result for same preprocessor input.
    // Return 4 symbol.
    fn get_class_suffix(&self) -> String {
        self.random_ident[1..=4].iter().collect::<String>()
    }
}

#[derive(Clone)]
pub struct ClassInfo {
    pub class_name: String,
    pub original_span: Option<proc_macro2::Span>,
}
impl From<String> for ClassInfo {
    fn from(class_name: String) -> Self {
        Self {
            class_name,
            original_span: None,
        }
    }
}

pub struct CssOutput {
    uniq_class: String,
    css_data: String,
    changed_classes: BTreeMap<String, ClassInfo>,
}

impl CssOutput {
    #[doc(hidden)]
    pub fn create_from_fields(
        uniq_class: String,
        css_data: String,
        changed_classes: BTreeMap<String, ClassInfo>,
    ) -> Self {
        Self {
            uniq_class,
            css_data,
            changed_classes,
        }
    }

    #[doc(hidden)]
    pub fn classes_list(&self) -> impl Iterator<Item = &str> {
        self.changed_classes.keys().map(|k| k.as_str())
    }

    pub fn style_string(&self) -> String {
        self.css_data.clone()
    }

    pub fn class_name(&self) -> &str {
        &self.uniq_class
    }
    pub fn class_suffix(&self) -> &str {
        &self.uniq_class[1..=4]
    }

    pub fn generate_css_module(&self, global: Option<Ident>) -> proc_macro2::TokenStream {
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

        #[allow(unused_mut)] //used in feature
        let mut changed_classes = self.changed_classes.clone();
        #[cfg(feature = "auto-snake-case")]
        {
            use inflector::cases::snakecase::to_snake_case;
            for (k, v) in &self.changed_classes {
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

        let mod_ident = global
            .as_ref()
            .cloned()
            .unwrap_or_else(|| quote::format_ident!("Css"));
        let index_impl = if cfg!(feature = "indexed-classes") {
            quote::quote! {
                impl<'a> std::ops::Index<&'a str> for #mod_ident {
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

        // TODO: find a way to warn on generated dead code (fields that wasn't accessed).
        let module_impl = quote::quote! {

                pub struct #mod_ident {
                    #(pub #field_classes: &'static str,)*
                    __kebab_styled: std::collections::BTreeMap<&'static str, &'static str>,
                }
                impl #mod_ident {
                    pub fn new() -> Self {
                        let mut map = std::collections::BTreeMap::new();
                        #(#kebab_map_init)*
                        Self {
                            #(#field_init)*
                            __kebab_styled: map,
                        }
                    }
                }
                #index_impl
        };
        if global.is_none() {
            quote::quote! {
                {
                    #module_impl
                    #mod_ident::new()
                }
            }
        } else {
            module_impl
        }
    }

    pub fn merge_to_string(styles: &[Self]) -> String {
        let mut result = String::new();
        for style in styles {
            result.push_str(&style.css_data);
        }
        result
    }
    /// Save multiple outputs to a single file.
    pub fn merge_to_file(styles: &[Self], file: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = std::fs::File::create(file)?;
        for style in styles {
            file.write_all(style.css_data.as_bytes())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn check_process_class_names() {
        let style = r#"
        .my-class {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);

        assert!(output.changed_classes["my-class"]
            .class_name
            .contains("my-class"));
        let output_css = format!(r#".my-class-{}{{color:red}}"#, output.class_suffix());
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn check_global_selector() {
        let style = r#"
        :global(.my-class) {
            color: red;
        }
        :global(b) {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let mut output_css = String::new();
        output_css.push_str(&r#".my-class{color:red}"#);
        output_css.push_str(&r#"b{color:red}"#);
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn check_deep_selector() {
        let style = r#"
        :deep(.my-class) {
            color: red;
        }
        :deep(b) {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let suffix = output.class_suffix();
        let mut output_css = String::new();
        output_css.push_str(&format!(r#".my-class-{suffix}{{color:red}}"#));
        output_css.push_str(&r#"b{color:red}"#);
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn check_process_types_ids() {
        let style = r#"
        element {
            color: red;
        }
        #my-id {
            color: red;
        }
        type#with-id {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let uniq_class = output.class_name();
        let mut output_css = String::new();
        output_css.push_str(&format!(r#"element.{uniq_class}{{color:red}}"#));
        output_css.push_str(&format!(r#"#my-id.{uniq_class}{{color:red}}"#));
        output_css.push_str(&format!(r#"type#with-id.{uniq_class}{{color:red}}"#));
        assert_eq!(output.css_data, output_css)
    }

    #[test]
    fn check_child_class() {
        let style = r#"
        type#with-id .class1{
            color: red;
        }
        element > .child {
            color: red;
        }
        .parent > element2 {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let uniq_class = output.class_name();
        let suffix = output.class_suffix();
        let mut output_css = String::new();
        output_css.push_str(&format!(
            r#"type#with-id.{uniq_class} .class1-{suffix}{{color:red}}"#
        ));

        output_css.push_str(&format!(
            r#"element.{uniq_class}>.child-{suffix}{{color:red}}"#
        ));

        output_css.push_str(&format!(
            r#".parent-{suffix}>element2.{uniq_class}{{color:red}}"#
        ));

        assert_eq!(output.css_data, output_css)
    }

    #[test]
    fn check_components_parsing() {
        let style = r#"
        type#with-id.class1[attribute=value]{
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let suffix = output.class_suffix();
        let mut output_css = String::new();
        output_css.push_str(&format!(
            r#"type#with-id.class1-{suffix}[attribute=value]{{color:red}}"#
        ));

        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn check_child_class2() {
        let style = r#"
        .parent > element2 {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let uniq_class = output.class_name();
        let suffix = output.class_suffix();
        let mut output_css = String::new();

        output_css.push_str(&format!(
            r#".parent-{suffix}>element2.{uniq_class}{{color:red}}"#
        ));

        assert_eq!(output.css_data, output_css)
    }

    #[test]
    fn check_mixed_types_ids_classes() {
        let style = r#"
        element, .class1 {
            color: red;
        }
        #my-id.class2 {
            color: red;
        }
        type#with-id .class3{
            color: red;
        }
        element2 {
            color: red;
        }
        .my-class {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let uniq_class = output.class_name();
        let suffix = output.class_suffix();
        let mut output_css = String::new();
        output_css.push_str(&format!(
            r#"element.{uniq_class},.class1-{suffix}{{color:red}}"#
        ));

        output_css.push_str(&format!(r#"#my-id.class2-{suffix}{{color:red}}"#));

        output_css.push_str(&format!(
            r#"type#with-id.{uniq_class} .class3-{suffix}{{color:red}}"#
        ));
        output_css.push_str(&format!(r#"element2.{uniq_class}{{color:red}}"#));
        output_css.push_str(&format!(r#".my-class-{suffix}{{color:red}}"#));
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn complex_deep_global_combination() {
        let style = r#"
        :global(.my-class) {
            color: red;
        }
        :deep(.my-class2) {
            color: red;
        }
        :global(:deep(.my-class3)) {
            color: red;
        }
        :deep(:global(.my-class4)) {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let suffix = output.class_suffix();
        let output_css = format!(
            r#".my-class{{color:red}}.my-class2-{suffix}{{color:red}}.my-class3{{color:red}}.my-class4{{color:red}}"#
        );
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn complex_selector_in_deep() {
        let style = r#"
        :deep(.my-class) {
            color: red;
        }
        :deep(.my-class2 .my-class3) {
            color: red;
        }
        :deep(.my-class4 > .my-class5) {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let suffix = output.class_suffix();
        let output_css = format!(
            r#".my-class-{suffix}{{color:red}}.my-class2-{suffix} .my-class3-{suffix}{{color:red}}.my-class4-{suffix}>.my-class5-{suffix}{{color:red}}"#
        );
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn id_after_global() {
        let style = r#"
        :global(.my-class)#my-id {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let mut output_css = String::new();
        output_css.push_str(&format!(r#".my-class#my-id{{color:red}}"#));
        assert_eq!(output.css_data, output_css)
    }
    #[test]
    fn id_after_deep() {
        let style = r#"
        :deep(.my-class)#my-id {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let suffix = output.class_suffix();
        let mut output_css = String::new();
        output_css.push_str(&format!(r#".my-class-{suffix}#my-id{{color:red}}"#));
        assert_eq!(output.css_data, output_css)
    }

    #[test]
    fn complex_selector_in_global() {
        let style = r#"
        :global(.my-class) {
            color: red;
        }
        :global(.my-class2 .my-class3) {
            color: red;
        }
        :global(.my-class4 > .my-class5) {
            color: red;
        }
        "#;
        let output = super::CssProcessor::process_style(style);
        let output_css = format!(
            r#".my-class{{color:red}}.my-class2 .my-class3{{color:red}}.my-class4>.my-class5{{color:red}}"#
        );
        assert_eq!(output.css_data, output_css)
    }
}
