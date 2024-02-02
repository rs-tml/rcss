//! Implementation of @rcss(..) at rule
//!
//! The aims of this at-rule is to:
//! - Merge mod and inline implementation of rcss macro.
//! - Allow changing parsing behaviour.

use std::{fmt::Debug, str::FromStr};

use lightningcss::{
    traits::AtRuleParser,
    visitor::{Visit, VisitTypes, Visitor},
};
use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use thiserror::Error;

use syn::{ItemStruct, Path, Token};
pub struct RcssAtRuleParser;

#[derive(Clone)]
pub enum RcssAtRuleConfig {
    Struct(ItemStruct),
    Extend(Path),
}
impl Debug for RcssAtRuleConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RcssAtRuleConfig::Struct(item_mod) => write!(f, "Mod{}", item_mod.to_token_stream()),
            RcssAtRuleConfig::Extend(path) => write!(f, "Extend{}", path.to_token_stream()),
        }
    }
}
impl RcssAtRuleConfig {
    pub fn from_token_stream(tokens: TokenStream) -> Result<Self, AtRuleError> {
        let mut iter = tokens.clone().into_iter();

        if matches!(iter.next(), Some(TokenTree::Ident(i)) if i.to_string() == "extend") {
            let tokens = iter.collect();
            let result = syn::parse2::<Path>(tokens)?;
            Ok(RcssAtRuleConfig::Extend(result))
        } else {
            let mut tokens = tokens;
            // append semicolon, to statisfy syn::parse2::<ItemStruct>
            Token![;](proc_macro2::Span::call_site()).to_tokens(&mut tokens);

            let mut result = syn::parse2::<ItemStruct>(tokens)?;
            // TODO: Check instead of adding.
            result.fields = syn::Fields::Unit;
            result.semi_token = None;
            Ok(RcssAtRuleConfig::Struct(result))
        }
    }
}

#[derive(Debug, Error)]
pub enum AtRuleError {
    #[error("Unexpected at-rule, expected only rcss extension")]
    UnexpectedAtRule,
    #[error("Rcss rule has no block")]
    UnexpectedBlock,
    #[error("Failed to parse rcss rule as syn expression")]
    ErrorFromSyn(#[from] syn::Error),
    #[error("Failed to parse rcss rule as rust code")]
    TokenStreamError(#[from] proc_macro2::LexError),
}

impl<'i> AtRuleParser<'i> for RcssAtRuleParser {
    type Prelude = RcssAtRuleConfig;
    type AtRule = RcssAtRuleConfig;
    type Error = AtRuleError;

    fn parse_prelude<'t>(
        &mut self,
        name: cssparser::CowRcStr<'i>,
        input: &mut cssparser::Parser<'i, 't>,
        _options: &lightningcss::stylesheet::ParserOptions<'_, 'i>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        if name != "rcss" {
            return Err(input.new_custom_error(AtRuleError::UnexpectedAtRule));
        }
        input.expect_parenthesis_block()?;
        let stream = input.parse_nested_block(|input| {
            let start = input.state().position();
            while let Ok(_v) = input.next() {
                // skip tokens to parse them later with syn
            }

            Ok(input.slice_from(start))
        })?;

        let stream = stream.trim();

        let tokens = proc_macro2::TokenStream::from_str(stream)
            .map_err(|e| dbg!(input.new_custom_error(e)))?;

        RcssAtRuleConfig::from_token_stream(tokens).map_err(|e| input.new_custom_error(e))
    }

    fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start: &cssparser::ParserState,
        input: &mut cssparser::Parser<'i, 't>,
        _options: &lightningcss::stylesheet::ParserOptions<'_, 'i>,
        _is_nested: bool,
    ) -> Result<Self::AtRule, cssparser::ParseError<'i, Self::Error>> {
        Err(input.new_custom_error(AtRuleError::UnexpectedBlock))
    }

    fn rule_without_block(
        &mut self,
        prelude: Self::Prelude,
        _start: &cssparser::ParserState,
        _options: &lightningcss::stylesheet::ParserOptions<'_, 'i>,
        _is_nested: bool,
    ) -> Result<Self::AtRule, ()> {
        Ok(prelude)
    }
}

impl lightningcss::traits::ToCss for RcssAtRuleConfig {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        let args = match self {
            RcssAtRuleConfig::Struct(item_mod) => {
                // Don't use neither semicolon, nor block in ItemStruct
                let mut tokens = TokenStream::new();
                tokens.append_all(item_mod.attrs.iter());
                item_mod.vis.to_tokens(&mut tokens);
                item_mod.struct_token.to_tokens(&mut tokens);
                item_mod.ident.to_tokens(&mut tokens);
                tokens
            }
            RcssAtRuleConfig::Extend(path) => path.to_token_stream(),
        };
        dest.write_str(&format!("@rcss({args});"))
    }
}

impl<'i, V: Visitor<'i, RcssAtRuleConfig>> Visit<'i, RcssAtRuleConfig, V> for RcssAtRuleConfig {
    const CHILD_TYPES: VisitTypes = VisitTypes::empty();
    fn visit_children(&mut self, _: &mut V) -> Result<(), V::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use lightningcss::{rules::CssRule, traits::ToCss};
    use quote::ToTokens;

    use super::RcssAtRuleParser;

    #[test]
    fn check_at_rule_mod() {
        let input = r#"
            @rcss(mod my_mod);
            
            .my-class {
                color: red;
            }
            
        "#;
        let stylesheet = lightningcss::stylesheet::StyleSheet::parse_with(
            input,
            Default::default(),
            &mut RcssAtRuleParser,
        )
        .unwrap();
        let rule = stylesheet.rules.0.into_iter().next().unwrap();

        match &rule {
            CssRule::Custom(super::RcssAtRuleConfig::Struct(item_mod)) => {
                assert_eq!(item_mod.to_token_stream().to_string(), "mod my_mod ;")
            }
            _ => unreachable!(),
        }
        let output = rule.to_css_string(Default::default()).unwrap();
        assert_eq!(output, "@rcss(mod my_mod);");
    }

    #[test]
    fn check_at_rule_extend() {
        let input = r#"
            @rcss(extend ::path::to::my_mod);
            
            .my-class {
                color: red;
            }
            
        "#;
        let stylesheet = lightningcss::stylesheet::StyleSheet::parse_with(
            input,
            Default::default(),
            &mut RcssAtRuleParser,
        )
        .unwrap();
        let rule = stylesheet.rules.0.into_iter().next().unwrap();
        match &rule {
            CssRule::Custom(super::RcssAtRuleConfig::Extend(path)) => {
                assert_eq!(
                    path.to_token_stream().to_string(),
                    ":: path :: to :: my_mod"
                )
            }
            _ => unreachable!(),
        }
        let output = rule.to_css_string(Default::default()).unwrap();
        assert_eq!(output, "@rcss(:: path :: to :: my_mod);");
    }
}
