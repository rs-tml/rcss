use super::{CssStyleProcessor, FragmentVisitor, SelectorFragment};
use cssparser::ToCss;
use lightningcss::{
    selector::Selector,
    stylesheet::{ParserOptions, PrinterOptions},
    visit_types,
    visitor::{Visit, VisitTypes},
};
use parcel_selectors::parser::Component;

pub struct Preprocessor<'i> {
    style: lightningcss::stylesheet::StyleSheet<'i, 'i>,
}
impl SelectorFragment for Selector<'_> {
    fn append_new_class(&mut self, class: &str) {
        let class = class.to_string();
        self.append(Component::Class(class.into()))
    }
}

impl<'i> CssStyleProcessor<'i> for Preprocessor<'i> {
    type Fragment = Selector<'i>;

    fn load_style(style: &'i str) -> Self {
        let style =
            lightningcss::stylesheet::StyleSheet::parse(style, ParserOptions::default()).unwrap();
        Self { style }
    }

    fn visit_modify<F>(&mut self, visitor: F)
    where
        F: FragmentVisitor<Fragment = Self::Fragment>,
    {
        struct LightningVisitor<F> {
            visitor: F,
        }
        impl<'i, F> lightningcss::visitor::Visitor<'i> for LightningVisitor<F>
        where
            F: FragmentVisitor<Fragment = Selector<'i>>,
        {
            type Error = ();
            fn visit_types(&self) -> VisitTypes {
                visit_types!(SELECTORS)
            }
            fn visit_selector(&mut self, fragment: &mut Selector<'i>) -> Result<(), Self::Error> {
                self.visitor.visit_selector_fragment(fragment);

                for component in fragment.iter_mut_raw_match_order() {
                    match component {
                        Component::Class(ref mut class) => {
                            let source: String = class.to_css_string();
                            let mut cloned = source.clone();
                            self.visitor.visit_each_class(&mut cloned);
                            if cloned != source {
                                let result: cssparser::CowRcStr = cloned.into();
                                *class = result.into();
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
        }
        self.style.visit(&mut LightningVisitor { visitor }).unwrap();
    }

    fn to_string(&self) -> String {
        let mut options = PrinterOptions::default();
        options.minify = true;
        self.style.to_css(options).unwrap().code
    }
}
