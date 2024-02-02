// TODO:
// - [ ] Add support for css interpolation (usefull for theming, can be used with css custom properties).
// - [ ] Procss preprocessor.
// - [ ] :deep pseudo-elemenet support

use std::{collections::BTreeMap, io::Write, path::Path};

use lightningcss::{
    stylesheet::{ParserOptions, PrinterOptions},
    visitor::Visit,
};
use rand::{distributions::Distribution, seq::SliceRandom, Rng, SeedableRng};
use rcss_at_rule::{RcssAtRuleConfig, RcssAtRuleParser};

pub mod rcss_at_rule;
pub mod visitor;
pub use visitor::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct CssProcessor<'i> {
    style: lightningcss::stylesheet::StyleSheet<'i, 'i, RcssAtRuleConfig>,
    // use array instead of string to avoid heap allocation.
    random_ident: [char; 7],
}
impl<'src> CssProcessor<'src> {
    // TODO: Handle error
    fn new(style: &'src str) -> Result<Self> {
        let this = Self {
            random_ident: Self::init_random_class(style),
            style: lightningcss::stylesheet::StyleSheet::parse_with(
                style,
                ParserOptions::default(),
                &mut RcssAtRuleParser,
            )
            .map_err(|e| Error::ParseError(format!("{:?}", e)))?,
        };
        Ok(this)
    }
    pub fn process_style(style: &'src str) -> Result<CssOutput> {
        let mut this = Self::new(style)?;
        this.process_style_inner()
    }

    fn process_style_inner<'a>(&mut self) -> Result<CssOutput> {
        // Create visitor that will modify class names, but will not modify css rules.
        let suffix = self.get_class_suffix();
        let mut visitor = visitor::SelectorVisitor {
            append_class: self.get_scoped_class(),
            class_modify: Box::new(move |class| format!("{class}-{suffix}")),
            collect_classes: BTreeMap::new(),
            declare: None,
            extend: None,
            state: Default::default(),
        };
        self.style.visit(&mut visitor)?;
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
        Ok(CssOutput {
            uniq_class: visitor.append_class,
            css_data: self
                .style
                .to_css(PrinterOptions {
                    minify: true,
                    ..Default::default()
                })
                .unwrap()
                .code,
            declare: visitor.declare,
            extend: visitor.extend,
            changed_classes,
        })
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

#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub struct CssOutput {
    uniq_class: String,
    css_data: String,
    declare: Option<syn::ItemStruct>,
    extend: Option<syn::Path>,
    changed_classes: BTreeMap<String, ClassInfo>,
}

impl CssOutput {
    #[doc(hidden)]
    pub fn create_from_fields(
        uniq_class: String,
        css_data: String,
        declare: Option<syn::ItemStruct>,
        extend: Option<syn::Path>,
        changed_classes: BTreeMap<String, ClassInfo>,
    ) -> Self {
        Self {
            uniq_class,
            css_data,
            declare,
            extend,
            changed_classes,
        }
    }

    #[doc(hidden)]
    pub fn classes_list(&self) -> impl Iterator<Item = &str> {
        self.changed_classes.keys().map(|k| k.as_str())
    }
    /// Returns map of changed classes.
    pub fn classes_map(&self) -> &BTreeMap<String, ClassInfo> {
        &self.changed_classes
    }

    /// Returns mod name if css should emit mod instead of inline struct.
    pub fn declare(&self) -> Option<syn::ItemStruct> {
        self.declare.clone()
    }
    /// Returns path to mod if css should extend existing css in mod instead of creating one from scratch.
    pub fn extend(&self) -> Option<syn::Path> {
        self.extend.clone()
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
        let output = super::CssProcessor::process_style(style).unwrap();

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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
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
        let output = super::CssProcessor::process_style(style).unwrap();
        let output_css = format!(
            r#".my-class{{color:red}}.my-class2 .my-class3{{color:red}}.my-class4>.my-class5{{color:red}}"#
        );
        assert_eq!(output.css_data, output_css)
    }
}
