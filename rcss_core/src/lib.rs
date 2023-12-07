// TODO:
// - [ ] Add support for css interpolation (usefull for theming, can be used with css custom properties).
// - [ ] Procss preprocessor.
// - [ ] :deep pseudo-elemenet support

use std::{collections::BTreeMap, io::Write, path::Path};

use proc_macro2::{Ident, Literal};
use rand::{distributions::Distribution, seq::SliceRandom, Rng, SeedableRng};

/// Helper module for writing proc-macros.
#[cfg(feature = "macro-helper")]
pub mod macro_helper;

#[cfg(feature = "lightningcss")]
pub mod lightning_css;

#[cfg(not(feature = "lightningcss"))]
#[path = "feature_not_activated.rs"]
pub mod lightning_css;

#[path = "feature_not_activated.rs"]
pub mod pro_css;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum CssPreprocessor {
    /// Browsergrade lightningcss
    LightningCss,
    /// lightweight procss
    Procss,
    /// stylers_core from leptos ecosystem
    StylersCore,
}

// Define is it scoped or css modules api.
// Whenever preprocessor should modify existing class names, or generate new ones for scope
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum CssEmbeding {
    /// Css modules like api.
    /// It generate struct with css classes as fields.
    CssModules,
    /// Scoped css.
    /// Uses random class as separator for different scopes.
    Scoped,
    /// No preprocessing, just output css as is.
    Global,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct CssProcessor {
    preprocessor: CssPreprocessor,
    embeding: CssEmbeding,
    // use array instead of string to avoid heap allocation.
    random_ident: [char; 7],
}

/// Helper trait to work with css selectors.
/// It is used to modify single selector from css rule.
/// Preserve original order of classes, ids, pseudos, etc.
///
/// Example:
/// ```compile_fail
/// let mut selectors: Vec<_> = SomeAbstractSelectorFragment::new_list(".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id");
/// for fragment in selectors {
///     fragment.append_new_class("bar");
/// }
/// assert_eq!(SomeAbstractSelectorFragment::list_to_css(&selector),
///            ".my-class[data-attr] > .my-class2.bar, div.foo:pseudo.bar, .asd#id.bar");
/// ```
trait SelectorFragment {
    // Add new class to selector fragment.
    fn append_new_class(&mut self, class: &str);
}

trait FragmentVisitor {
    type Fragment: SelectorFragment;

    // Visit each class in selector, and modify it.
    fn visit_each_class(&mut self, _class: &mut String) {}
    // Visit each selector fragment and modify it through SelectorFragment api
    fn visit_selector_fragment(&mut self, _selector: &mut Self::Fragment) {}
}

struct EmptyVisitor<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: SelectorFragment> FragmentVisitor for EmptyVisitor<T> {
    type Fragment = T;
}

trait CssStyleProcessor<'i> {
    type Fragment: SelectorFragment;

    fn load_style(style: &'i str) -> Self;
    fn visit_modify<F>(&mut self, visitor: F)
    where
        F: FragmentVisitor<Fragment = Self::Fragment>;
    fn to_string(&self) -> String;
}

impl CssProcessor {
    pub fn new(preprocessor: CssPreprocessor, embeding: CssEmbeding) -> Self {
        Self {
            preprocessor,
            embeding,
            random_ident: ['_'; 7],
        }
    }
    pub fn process_style(&mut self, style: &str) -> CssOutput {
        self.init_random_class(style);
        match (self.preprocessor, self.embeding) {
            (CssPreprocessor::StylersCore, CssEmbeding::Scoped) => {
                self.process_style_with_stylers(style)
            }
            (CssPreprocessor::StylersCore, _) => {
                panic!("StylersCore preprocessor supports only scoped embeding")
            }
            (CssPreprocessor::LightningCss, _) => {
                self.process_style_with_preprocessor::<lightning_css::Preprocessor>(style)
            }
            (CssPreprocessor::Procss, _) => {
                self.process_style_with_preprocessor::<pro_css::Preprocessor>(style)
            }
        }
    }

    fn collect_changed_classes<'a, V, T>(processor: &mut T, visitor: V) -> BTreeMap<String, String>
    where
        V: FragmentVisitor<Fragment = T::Fragment>,
        T: CssStyleProcessor<'a>,
    {
        struct ChangesCollector<'a, V: FragmentVisitor> {
            changed_classes: &'a mut BTreeMap<String, String>,
            visitor: V,
        }
        impl<'a, V: FragmentVisitor> FragmentVisitor for ChangesCollector<'a, V> {
            type Fragment = V::Fragment;

            fn visit_each_class(&mut self, class: &mut String) {
                let cloned = class.clone();
                self.visitor.visit_each_class(class);
                if cloned != *class {
                    self.changed_classes.insert(cloned, class.clone());
                }
            }
            fn visit_selector_fragment(&mut self, selector: &mut Self::Fragment) {
                self.visitor.visit_selector_fragment(selector);
            }
        }

        let mut changed_classes = BTreeMap::new();
        processor.visit_modify(ChangesCollector {
            changed_classes: &mut changed_classes,
            visitor,
        });
        changed_classes
    }

    fn process_style_with_preprocessor<'a, T: CssStyleProcessor<'a>>(
        &self,
        style: &'a str,
    ) -> CssOutput {
        // Create visitor that will modify class names, but will not modify css rules.
        let visitor = match self.embeding {
            CssEmbeding::CssModules => {
                OneOf::First(self.css_modules_universal_visitor::<T::Fragment>())
            }
            CssEmbeding::Scoped => OneOf::Second(self.scoped_universal_visitor::<T::Fragment>()),
            CssEmbeding::Global => OneOf::Third(self.global_universal_visitor::<T::Fragment>()),
        };

        let mut processor = T::load_style(style);

        let changed_classes = Self::collect_changed_classes(&mut processor, visitor)
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        CssOutput {
            uniq_class: self.generate_random_class(true),
            css_data: processor.to_string(),
            changed_classes,
        }
    }
    // Modify each class with random postfix.
    fn css_modules_universal_visitor<'a, S: SelectorFragment>(
        &'a self,
    ) -> impl FragmentVisitor<Fragment = S> + 'a
    where
        S: 'a,
    {
        struct CssModulesVisitor<S> {
            random_class_suffix: String,
            _marker: std::marker::PhantomData<S>,
        }
        impl<S: SelectorFragment> FragmentVisitor for CssModulesVisitor<S> {
            type Fragment = S;

            fn visit_each_class(&mut self, class: &mut String) {
                *class = format!("{}-{}", class, self.random_class_suffix);
            }
        }
        CssModulesVisitor {
            random_class_suffix: self.generate_random_class(false),
            _marker: std::marker::PhantomData,
        }
    }
    // Add random class as scope separator.
    fn scoped_universal_visitor<S: SelectorFragment>(&self) -> impl FragmentVisitor<Fragment = S> {
        struct ScopedVisitor<S> {
            random_class: String,
            _marker: std::marker::PhantomData<S>,
        }
        impl<S: SelectorFragment> FragmentVisitor for ScopedVisitor<S> {
            type Fragment = S;

            fn visit_selector_fragment(&mut self, selector: &mut Self::Fragment) {
                selector.append_new_class(&self.random_class);
            }
        }
        ScopedVisitor {
            random_class: self.generate_random_class(true),
            _marker: std::marker::PhantomData,
        }
    }

    fn global_universal_visitor<S: SelectorFragment>(&self) -> impl FragmentVisitor<Fragment = S> {
        // Do nothing just output css without modification.
        EmptyVisitor {
            _marker: std::marker::PhantomData,
        }
    }

    fn process_style_with_stylers(&self, style: &str) -> CssOutput {
        #[cfg(feature = "stylers")]
        {
            let class = stylers_core::Class::new(self.generate_random_class(true));
            let output_style = stylers_core::from_str(style, &class);

            CssOutput {
                uniq_class: class.as_name().to_owned(),
                css_data: output_style,
                changed_classes: BTreeMap::new(),
            }
        }

        #[cfg(not(feature = "stylers"))]
        {
            let _ = style;
            panic!("StylersCore preprocessor was disabled")
        }
    }

    fn init_random_class(&mut self, style: &str) {
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
        self.random_ident = std::array::from_fn(|i| ident_vec[i]);
    }

    // Returns random class identifier.
    // Each call return same result for same preprocessor input.
    // Use flag scoped to define size of generated identifier.
    // If scoped is true, it will generate class name with 7 symbols with first
    // symbol as underscore, suitable for css modules.
    // If scoped is false, it will generate class name with 4 symbols, suitable for suffixes.
    fn generate_random_class(&self, scoped: bool) -> String {
        if scoped {
            self.random_ident.iter().collect::<String>()
        } else {
            self.random_ident[1..=4].iter().collect::<String>()
        }
    }
}

enum OneOf<T, U, Z> {
    First(T),
    Second(U),
    Third(Z),
}

impl<T, U, Z, Frag: SelectorFragment> FragmentVisitor for OneOf<T, U, Z>
where
    T: FragmentVisitor<Fragment = Frag>,
    U: FragmentVisitor<Fragment = Frag>,
    Z: FragmentVisitor<Fragment = Frag>,
{
    type Fragment = Frag;

    fn visit_each_class(&mut self, class: &mut String) {
        match self {
            OneOf::First(visitor) => visitor.visit_each_class(class),
            OneOf::Second(visitor) => visitor.visit_each_class(class),
            OneOf::Third(visitor) => visitor.visit_each_class(class),
        }
    }
    fn visit_selector_fragment(&mut self, selector: &mut Self::Fragment) {
        match self {
            OneOf::First(visitor) => visitor.visit_selector_fragment(selector),
            OneOf::Second(visitor) => visitor.visit_selector_fragment(selector),
            OneOf::Third(visitor) => visitor.visit_selector_fragment(selector),
        }
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
        self.changed_classes.iter().map(|(k, _v)| k.as_str())
    }

    pub fn to_string(&self) -> String {
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

    struct TestVisitor<T> {
        suffix: Option<String>,
        new_class: Option<String>,
        _marker: std::marker::PhantomData<T>,
    }

    impl<T> TestVisitor<T> {
        #[allow(dead_code)] //used in feature
        fn new_suffix(suffix: &str) -> Self {
            Self {
                suffix: Some(suffix.to_string()),
                new_class: None,
                _marker: std::marker::PhantomData,
            }
        }
        #[allow(dead_code)] //used in feature
        fn new_class(new_class: &str) -> Self {
            Self {
                suffix: None,
                new_class: Some(new_class.to_string()),
                _marker: std::marker::PhantomData,
            }
        }
    }
    impl<T> super::FragmentVisitor for TestVisitor<T>
    where
        T: super::SelectorFragment,
    {
        type Fragment = T;

        fn visit_each_class(&mut self, class: &mut String) {
            if let Some(suffix) = &self.suffix {
                *class = format!("{}-{}", class, suffix);
            }
        }
        fn visit_selector_fragment(&mut self, selector: &mut Self::Fragment) {
            if let Some(new_class) = &self.new_class {
                selector.append_new_class(new_class)
            }
        }
    }

    // Check that lightning css visitor can add suffix to eh class.
    #[cfg(feature = "lightningcss")]
    #[test]
    fn lightning_css_visitor_suffix() {
        use super::lightning_css::Preprocessor;
        use super::CssStyleProcessor;
        let mut processor = Preprocessor::load_style(
            ".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}",
        );
        processor.visit_modify(TestVisitor::new_suffix("bar"));
        assert_eq!(
            processor.to_string(),
            ".my-class-bar[data-attr]>.my-class2-bar,div.foo-bar:pseudo,.asd-bar#id{}"
        );
    }

    // Check that lightning css visitor can add new class to each selector.
    #[cfg(feature = "lightningcss")]
    #[test]
    fn lightning_css_visitor_scoped() {
        use super::lightning_css::Preprocessor;
        use super::CssStyleProcessor;
        let mut processor = Preprocessor::load_style(
            ".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}",
        );
        processor.visit_modify(TestVisitor::new_class("bar"));
        assert_eq!(
            processor.to_string(),
            ".my-class[data-attr]>.my-class2.bar,div.foo:pseudo.bar,.asd#id.bar{}"
        );
    }
    #[cfg(feature = "lightningcss")]
    #[test]
    fn lightning_css_check_attribute_selector() {
        use super::lightning_css::Preprocessor;
        use super::CssStyleProcessor;
        let mut processor = Preprocessor::load_style(".my-class[data-attr] {}");
        processor.visit_modify(TestVisitor::new_class("bar"));
        assert_eq!(processor.to_string(), ".my-class[data-attr].bar{}");
    }

    #[cfg(feature = "lightningcss")]
    #[test]
    fn check_processor_api() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor = CssProcessor {
            preprocessor: CssPreprocessor::LightningCss,
            embeding: CssEmbeding::Scoped,
            random_ident: ['_'; 7],
        };
        let style = ".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}";
        let output = processor.process_style(style);
        assert_eq!(
            output.to_string(),
            ".my-class[data-attr]>.my-class2._f1HPvp,div.foo:pseudo._f1HPvp,.asd#id._f1HPvp{}"
        );

        processor.embeding = CssEmbeding::CssModules;
        let output = processor.process_style(style);
        assert_eq!(
            output.to_string(),
            ".my-class-f1HP[data-attr]>.my-class2-f1HP,div.foo-f1HP:pseudo,.asd-f1HP#id{}"
        );
    }
    #[cfg(feature = "lightningcss")]
    #[test]
    fn check_that_class_random_changes() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor = CssProcessor::new(CssPreprocessor::LightningCss, CssEmbeding::Scoped);
        let style = ".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}";
        let output = processor.process_style(style);
        // note that random class is same as in check_processor_api test, because we use same input.
        assert_eq!(
            output.to_string(),
            ".my-class[data-attr]>.my-class2._f1HPvp,div.foo:pseudo._f1HPvp,.asd#id._f1HPvp{}"
        );

        // even small change in style should change random class.
        let style = ".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {color:red}";
        let output = processor.process_style(style);
        assert_eq!(
            output.to_string(),
            ".my-class[data-attr]>.my-class2._l3XxIL,div.foo:pseudo._l3XxIL,.asd#id._l3XxIL{color:red}"
        );
    }

    #[cfg(feature = "stylers")]
    #[test]
    fn stylers_check_scoped_api() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor = CssProcessor::new(CssPreprocessor::StylersCore, CssEmbeding::Scoped);
        // stylers can't handle correctly selector with attributes like .my-class[data-attr] > .my-class2
        // it produces ".my-class[data-attr]._f1HPvp._f1HPvp ._f1HPvp>.my-class2._f1HPvp"
        // instead of ".my-class[data-attr] > .my-class2._f1HPvp"
        // So we use different selector for this test.
        let style = ".my-class2, div.foo:pseudo, .asd#id {}";
        let output = processor.process_style(style);
        // order for pseudo classes is not preserved, but both is valid selectors.
        assert_eq!(
            output.to_string(),
            ".my-class2._lkhJfY,div.foo._lkhJfY:pseudo, .asd#id._lkhJfY{}"
        );
    }

    #[cfg(feature = "stylers")]
    #[test]
    #[should_panic = "StylersCore preprocessor supports only scoped embeding"]
    fn stylers_check_css_modules() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor =
            CssProcessor::new(CssPreprocessor::StylersCore, CssEmbeding::CssModules);
        let style = ".my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}";
        let _output = processor.process_style(style);
    }

    #[cfg(feature = "lightningcss")]
    #[test]
    #[cfg(not(feature = "auto-snake-case"))]
    fn check_css_module_generated_token_stream() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor =
            CssProcessor::new(CssPreprocessor::LightningCss, CssEmbeding::CssModules);
        let style =
            ".snaked_case_class .my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}";
        let output = processor.process_style(style);
        let css_module = output.generate_css_module(None);

        let index_impl = if cfg!(feature = "indexed-classes") {
            quote::quote! {
                impl<'a> std::ops::Index<&'a str> for Css {
                    type Output = str;
                    fn index(&self, index: &'a str) -> &Self::Output {
                        match index {
                            "asd" => self.asd,
                            "foo" => self.foo,
                            "snaked_case_class" => self.snaked_case_class,
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

        assert_eq!(
            css_module.to_string(),
            quote::quote! {{
                    pub struct Css {
                        pub asd: &'static str,
                        pub foo: &'static str,
                        pub snaked_case_class: &'static str,
                        __kebab_styled: std::collections::BTreeMap<&'static str, &'static str>,
                    }

                    impl Css {
                        pub fn new() -> Self {
                            let mut map = std::collections::BTreeMap::new();
                            map.insert("my-class", "my-class-mFQz");
                            map.insert("my-class2", "my-class2-mFQz");

                            Self {
                                asd: "asd-mFQz",
                                foo: "foo-mFQz",
                                snaked_case_class: "snaked_case_class-mFQz",
                                __kebab_styled: map,
                            }
                        }
                    }
                    #index_impl
                    Css::new()
                }
            }
            .to_string()
        );
    }
    #[cfg(feature = "lightningcss")]
    #[cfg(feature = "auto-snake-case")]
    #[test]
    fn check_css_module_generated_token_with_snake_conversion() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor =
            CssProcessor::new(CssPreprocessor::LightningCss, CssEmbeding::CssModules);
        let style =
            ".snaked_case_class .my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}";
        let output = processor.process_style(style);
        let css_module = output.generate_css_module(None);

        let index_impl = if cfg!(feature = "indexed-classes") {
            quote::quote! {
                impl<'a> std::ops::Index<&'a str> for Css {
                    type Output = str;
                    fn index(&self, index: &'a str) -> &Self::Output {
                        match index {
                            "asd" => self.asd,
                            "foo" => self.foo,
                            "my_class" => self.my_class,
                            "my_class_2" => self.my_class_2,
                            "snaked_case_class" => self.snaked_case_class,
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
        assert_eq!(
            css_module.to_string(),
            quote::quote! {{
                    pub struct Css {
                        pub asd: &'static str,
                        pub foo: &'static str,
                        pub my_class: &'static str,
                        pub my_class_2: &'static str,
                        pub snaked_case_class: &'static str,
                        __kebab_styled: std::collections::BTreeMap<&'static str, &'static str>,
                    }

                    impl Css {
                        pub fn new() -> Self {
                            let mut map = std::collections::BTreeMap::new();
                            map.insert("my-class", "my-class-mFQz");
                            map.insert("my-class2", "my-class2-mFQz");

                            Self {
                                asd: "asd-mFQz",
                                foo: "foo-mFQz",
                                my_class: "my-class-mFQz",
                                my_class_2: "my-class2-mFQz",
                                snaked_case_class: "snaked_case_class-mFQz",
                                __kebab_styled: map,
                            }
                        }
                    }
                    #index_impl

                    Css::new()
                }
            }
            .to_string()
        );
    }

    #[cfg(feature = "lightningcss")]
    #[cfg(feature = "auto-snake-case")]
    #[test]
    fn check_css_module_in_global_place() {
        use super::{CssEmbeding, CssPreprocessor, CssProcessor};
        let mut processor =
            CssProcessor::new(CssPreprocessor::LightningCss, CssEmbeding::CssModules);
        let style =
            ".snaked_case_class .my-class[data-attr] > .my-class2, div.foo:pseudo, .asd#id {}";
        let output = processor.process_style(style);
        let css_module = output.generate_css_module(Some(quote::format_ident!("MyModule")));

        let index_impl = if cfg!(feature = "indexed-classes") {
            quote::quote! {
                impl<'a> std::ops::Index<&'a str> for MyModule {
                    type Output = str;
                    fn index(&self, index: &'a str) -> &Self::Output {
                        match index {
                            "asd" => self.asd,
                            "foo" => self.foo,
                            "my_class" => self.my_class,
                            "my_class_2" => self.my_class_2,
                            "snaked_case_class" => self.snaked_case_class,
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
        assert_eq!(
            css_module.to_string(),
            quote::quote! {
                    pub struct MyModule {
                        pub asd: &'static str,
                        pub foo: &'static str,
                        pub my_class: &'static str,
                        pub my_class_2: &'static str,
                        pub snaked_case_class: &'static str,
                        __kebab_styled: std::collections::BTreeMap<&'static str, &'static str>,
                    }

                    impl MyModule {
                        pub fn new() -> Self {
                            let mut map = std::collections::BTreeMap::new();
                            map.insert("my-class", "my-class-mFQz");
                            map.insert("my-class2", "my-class2-mFQz");

                            Self {
                                asd: "asd-mFQz",
                                foo: "foo-mFQz",
                                my_class: "my-class-mFQz",
                                my_class_2: "my-class2-mFQz",
                                snaked_case_class: "snaked_case_class-mFQz",
                                __kebab_styled: map,
                            }
                        }
                    }
                    #index_impl
            }
            .to_string()
        );
    }
}
