use std::collections::BTreeMap;

use lightningcss::{
    properties::custom::{TokenList, TokenOrValue},
    rules::CssRule,
    selector::{Component, PseudoClass, Selector},
    stylesheet::{ParserOptions, PrinterOptions},
    traits::{ParseWithOptions, ToCss as _},
    values::ident::Ident,
    visit_types,
    visitor::{VisitTypes, Visitor},
};
use thiserror::Error;

use crate::rcss_at_rule::RcssAtRuleConfig;

pub(crate) struct SelectorVisitor {
    // Input:
    // Class name that should be appended to each selector without class.
    pub append_class: String,
    // Function that modify class_name to be unique.
    pub class_modify: Box<dyn FnMut(String) -> String>,
    // Output:
    // List of classes used in selectors.
    pub collect_classes: BTreeMap<String, String>,
    // If found macro should extend existing style from path.
    pub extend: Option<syn::Path>,
    // If found macro should emit mod instead of inline struct.
    pub declare: Option<syn::ItemStruct>,

    // State:
    pub state: SelectorState,
}

#[derive(Default, Clone, Debug)]
pub struct SelectorState {
    class_found: bool,
    global_selector: bool,
    deep_selector: bool,
}
impl SelectorState {
    fn handle_class(&mut self) {
        self.class_found = true;
    }
    fn handle_combinator(&mut self) {
        self.class_found = false;
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to print token as css")]
    PrintFailed(#[from] lightningcss::error::PrinterError),
    #[error("Failed to parse token as css selector")]
    ParseError(String),
    #[error("Not allowed token in selector list: {0}")]
    NotAllowedToken(String),
}

impl SelectorVisitor {
    fn token_list_to_selector<'i>(token_list: TokenList<'i>) -> Result<Selector<'i>, Error> {
        let mut result = String::new();
        for token in token_list.0 {
            match token {
                TokenOrValue::Angle(ref angle) => {
                    result.push_str(&angle.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::Token(ref token) => {
                    result.push_str(&token.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::Color(ref color) => {
                    result.push_str(&color.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::DashedIdent(ref ident) => {
                    result.push_str(&ident.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::Length(ref length) => {
                    result.push_str(&length.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::Resolution(ref resolution) => {
                    result.push_str(&resolution.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::Time(ref time) => {
                    result.push_str(&time.to_css_string(PrinterOptions::default())?)
                }
                TokenOrValue::Url(ref url) => {
                    result.push_str(&url.to_css_string(PrinterOptions::default())?)
                }
                _ => return Err(Error::NotAllowedToken(format!("{:?}", token))),
            }
        }
        let selector = Selector::parse_string_with_options(&result, ParserOptions::default())
            .map_err(|e| Error::ParseError(format!("{:?}", e)))?;
        use lightningcss::traits::IntoOwned;

        Ok(selector.into_owned())
    }
    fn try_modify_parts(&mut self, selectors: &mut Selector<'_>) -> Result<(), Error> {
        let class_name = self.append_class.clone();

        // Iterate over selector components
        // Split selector by its combinators
        let mut combinators = selectors
            .iter_raw_match_order()
            .rev()
            .filter_map(|x| x.as_combinator());
        let chunks = selectors
            .iter_raw_match_order()
            .as_slice()
            .split(|x| x.is_combinator())
            .rev();

        // Split selector by combinators
        let mut processed_selector = vec![];

        for chunk in chunks {
            if chunk.is_empty() {
                continue;
            }
            for part in chunk.into_iter().cloned() {
                // println!("component: {:?}, state:{state:?}", part, state = self.state);
                let part = match part {
                    Component::Class(mut class) => {
                        self.state.handle_class();
                        // Use css_module only outside of :global
                        if !self.state.global_selector {
                            self.modify_classes(&mut class)?;
                        }
                        // return back class to collection
                        Component::Class(class)
                    }
                    Component::NonTSPseudoClass(pseudo_class) => match pseudo_class {
                        // Lightningcss uses global type only with css_modules enabled
                        PseudoClass::Global { mut selector } => {
                            self.match_global(&mut processed_selector, &mut selector)?;
                            continue;
                        }
                        PseudoClass::CustomFunction { name, arguments } => {
                            if &*name == "deep" {
                                let mut selector =
                                    SelectorVisitor::token_list_to_selector(arguments.clone())?;

                                self.match_deep(&mut processed_selector, &mut selector)?;

                                continue;
                            }
                            if &*name == "global" {
                                let mut selector =
                                    SelectorVisitor::token_list_to_selector(arguments.clone())?;

                                self.match_global(&mut processed_selector, &mut selector)?;

                                continue;
                            }
                            Component::NonTSPseudoClass(PseudoClass::CustomFunction {
                                name,
                                arguments,
                            })
                        }
                        pseudo_class => Component::NonTSPseudoClass(pseudo_class),
                    },
                    rest => rest,
                };
                processed_selector.push(part)
            }
            if !self.state.class_found {
                Self::append_class(&self.state, &mut processed_selector, &class_name)?;
            }
            if let Some(combinator) = combinators.next() {
                processed_selector.push(Component::Combinator(combinator));
            }
            self.state.handle_combinator();
        }
        // println!("processed_selector: {:?}", processed_selector);
        *selectors = Selector::from(processed_selector);
        Ok(())
    }
    fn append_class(
        state: &SelectorState,
        selector_components: &mut Vec<Component>,
        class_name: &String,
    ) -> Result<(), Error> {
        // append class only if not in :deep and :global
        if !state.deep_selector && !state.global_selector {
            selector_components.push(Component::Class(class_name.clone().into()));
        }
        Ok(())
    }
    fn match_global<'i>(
        &mut self,
        selector_components: &mut Vec<Component<'i>>,
        selector: &mut Selector<'i>,
    ) -> Result<(), Error> {
        let mut child_state = self.state.clone();
        child_state.global_selector = true;
        std::mem::swap(&mut self.state, &mut child_state);
        self.visit_selector(selector)?;
        std::mem::swap(&mut self.state, &mut child_state);

        selector_components.extend(selector.iter_raw_parse_order_from(0).cloned());
        self.state.class_found = true;
        Ok(())
    }
    fn match_deep<'i>(
        &mut self,
        selector_components: &mut Vec<Component<'i>>,
        selector: &mut Selector<'i>,
    ) -> Result<(), Error> {
        let mut child_state = self.state.clone();
        child_state.deep_selector = true;
        std::mem::swap(&mut self.state, &mut child_state);
        self.visit_selector(selector)?;
        std::mem::swap(&mut self.state, &mut child_state);

        selector_components.extend(selector.iter_raw_parse_order_from(0).cloned());
        self.state.class_found = true;
        Ok(())
    }
    fn modify_classes(&mut self, class: &mut Ident<'_>) -> Result<(), Error> {
        let class_string = class.to_css_string(PrinterOptions::default())?;
        let modified = (*self.class_modify)(class_string.clone());
        self.collect_classes.insert(class_string, modified.clone());
        *class = modified.into();
        Ok(())
    }
    fn save_rcss_rule(&mut self, rcss_rule: RcssAtRuleConfig) {
        // TODO: Emit error on multiple rcss rules
        match rcss_rule {
            RcssAtRuleConfig::Struct(item_struct) => self.declare = Some(item_struct),
            RcssAtRuleConfig::Extend(path) => self.extend = Some(path),
        }
    }
}
impl<'i> lightningcss::visitor::Visitor<'i, crate::rcss_at_rule::RcssAtRuleConfig>
    for SelectorVisitor
{
    type Error = Error;
    fn visit_types(&self) -> VisitTypes {
        visit_types!(SELECTORS | RULES)
    }

    fn visit_selector(&mut self, fragment: &mut Selector<'i>) -> Result<(), Self::Error> {
        // println!("fragment: {:?}", fragment);
        self.state.class_found = false;
        self.try_modify_parts(fragment)?;

        Ok(())
    }
    fn visit_rule(
        &mut self,
        rule: &mut CssRule<'i, crate::rcss_at_rule::RcssAtRuleConfig>,
    ) -> Result<(), Self::Error> {
        match rule {
            CssRule::Custom(rcss) => {
                self.save_rcss_rule(rcss.clone());
                *rule = CssRule::Ignored;
            }
            rule => {
                use lightningcss::visitor::Visit;
                rule.visit_children(self)?;
            }
        }
        Ok(())
    }
}
