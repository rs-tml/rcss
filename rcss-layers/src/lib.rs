//! Layered CSS representation
//! Uses information from `ScopeChain` or one that user provide to build a layered representation of styles.
//!
//! Example:
//! ```no_compile
//! use crate::*;
//! let mut chain = LayeredCss::new();
//! chain.add_style_from_parts("root", 0, "root", "style1{color:red}");
//! chain.add_style_from_parts("root", 1, "layer2", "style2{color:blue}");
//! chain.add_style_from_parts("root", 2, "layer3", "style3{color:green}");
//! let representation = chain
//!       .styles
//!       .get("root")
//!       .unwrap()
//!       .render(false, "root")
//!       .unwrap();
//! let mut expectation = String::from("@layer root,layer2,layer3;");
//! expectation.push_str("@layer root{style1{color:red}}");
//! expectation.push_str("@layer layer2{style2{color:blue}}");
//! expectation.push_str("@layer layer3{style3{color:green}}");
//! assert_eq!(representation, expectation);
//! ```
//!
use std::{borrow::Cow, collections::BTreeMap};

/// Identifier of scoped style.
/// rcss uses `&'static str` as scope_id but we use `Cow<'static, str>` to support dynamic extension.
pub type ScopeId = Cow<'static, str>;
/// Style representation.
/// rcss uses `&'static str` as style representation but we use `Cow<'static, str>` to support dynamic extension.
pub type Style = Cow<'static, str>;
/// Order of layers is determined by how far a layer is from root.
pub type Order = u32;

#[derive(Debug, Default, Clone)]
pub struct LayeredCss {
    /// We store each chains in a groups
    /// with root_scope_id as identifier of the group.
    pub styles: BTreeMap<ScopeId, StyleTree>,
}
impl LayeredCss {
    pub fn new() -> Self {
        Self {
            styles: BTreeMap::new(),
        }
    }
    pub fn root_scope_exist(&self, root_scope_id: impl Into<ScopeId>) -> bool {
        self.styles.contains_key(&root_scope_id.into())
    }
    pub fn layer_exist_in_root_scope(
        &self,
        root_scope_id: impl Into<ScopeId>,
        layer_scope_id: impl Into<ScopeId>,
    ) -> bool {
        let root_scope_id = root_scope_id.into();
        let layer_scope_id = layer_scope_id.into();
        self.styles
            .get(&root_scope_id)
            .map(|style_tree| style_tree.uniq_layers.contains_key(&layer_scope_id))
            .unwrap_or(false)
    }

    /// Add new style layer to root_scope_id.
    /// Returns false if layer already exists.
    pub fn add_style_from_parts(
        &mut self,
        root_scope_id: impl Into<ScopeId>,
        order_in_chain: Order,
        layer_scope_id: impl Into<ScopeId>,
        style: impl Into<Style>,
    ) -> bool {
        let root_scope_id = root_scope_id.into();
        let layer_scope_id = layer_scope_id.into();
        let style = style.into();
        let layers_of_chain = &mut self
            .styles
            .entry(root_scope_id)
            .or_insert_with(Default::default)
            .uniq_layers;
        if let Some((order, _)) = layers_of_chain.get(&layer_scope_id) {
            debug_assert_eq!(*order, order_in_chain);
            return false;
        }
        let res = layers_of_chain
            .insert(layer_scope_id, (order_in_chain, style))
            .is_none();
        debug_assert!(res);
        true
    }
    /// Add new style layer.
    /// Uses `ScopeCommon` implementation to retrieve layer_scope_id and style.
    /// Returns false if layer already exists.
    #[cfg(feature = "rcss_enable")]
    pub fn add_style_with_order<T>(&mut self, root_scope_id: ScopeId, order_in_chain: Order) -> bool
    where
        T: rcss::ScopeCommon,
    {
        self.add_style_from_parts(root_scope_id, order_in_chain, T::SCOPE_ID, T::STYLE)
    }

    /// Add chain of styles to registry.
    /// Uses information from `ScopeChain` implementation,
    /// to retrieve scope_id of each layer and its style.
    /// Automatically detects root_scope_id and sets order based on distance from root.
    ///
    /// Return true if any of the layers is new.
    #[cfg(feature = "rcss_enable")]
    pub fn add_style_chain<C>(&mut self, ts_chain: &C) -> bool
    where
        C: rcss::extend::in_chain_ops::ScopeChainOps,
    {
        let root_scope_id = ts_chain.root_scope_id();
        // for_each is provide (scope_id, style) starting from bottom of the chain.
        // So we reverse it to get order from root.
        let mut chain = vec![];

        ts_chain.for_each(|scope_id, style| {
            chain.push((scope_id, style));
        });

        debug_assert_eq!(
            chain.last().expect("Chain is empty").0,
            ts_chain.root_scope_id()
        );
        let mut any_update = false;

        for (order, (scope_id, style)) in chain.into_iter().rev().enumerate() {
            any_update |= self.add_style_from_parts(root_scope_id, order as Order, scope_id, style);
        }
        any_update
    }
}
/// Store chain of extensions.
#[derive(Default, Clone, Debug)]
pub struct StyleTree {
    /// Style by id of its layer
    /// We store it in a map[ScopeId => _] and not in map[Order => _] because
    /// we want to prevent usage of same style in different chains.
    pub uniq_layers: BTreeMap<ScopeId, (Order, Style)>,
}

impl StyleTree {
    /// Returns None if root_scope_id is not registered.
    /// If always_output_layer is true, it will output layered css even when there only one layer.
    /// If always_output_layer is false, for single layer it will return plain css without layers.
    pub fn render(
        &self,
        always_output_layer: bool,
        root_scope_id: impl Into<ScopeId>,
    ) -> Option<String> {
        let root_scope_id = root_scope_id.into();
        let root_scope = self.uniq_layers.get(&root_scope_id)?.clone();

        debug_assert_eq!(root_scope.0, 0, "Root layer must have order 0");

        let mut rest = self
            .uniq_layers
            .iter()
            .filter(|(layer, _)| **layer != root_scope_id)
            .peekable();

        if !always_output_layer && rest.peek().is_none() {
            // if no other layers just return plain css without layers.
            return Some(root_scope.1.to_string());
        }
        let mut ordered_layers: Vec<_> = rest
            .map(|(layer, (order, style))| (order, layer, style))
            .collect();
        ordered_layers.sort_by_key(|(order, _, _)| *order);
        ordered_layers.insert(0, (&root_scope.0, &root_scope_id, &root_scope.1));

        let mut first: bool = true;
        let mut header = String::from("@layer ");

        for (_, scope_id, _) in &ordered_layers {
            if !first {
                header.push(',');
            }
            header.push_str(scope_id);
            first = false;
        }
        header.push(';');

        let mut style = header;

        // Push all layer declaration to header
        for (_, scope_id, layer_impl) in ordered_layers {
            style.push_str("@layer ");
            style.push_str(scope_id);
            style.push_str("{");
            style.push_str(layer_impl);
            style.push_str("}");
        }
        Some(style)
    }
}

#[cfg(test)]
#[cfg(feature = "rcss_enable")]
mod test {
    use rcss::ScopeCommon;

    use super::*;
    #[test]
    fn check_raw_api() {
        let mut chain = LayeredCss::new();
        assert!(chain.add_style_from_parts("root", 0, "root", "style1{color:red}"));
        assert!(chain.add_style_from_parts("root", 1, "layer2", "style2{color:blue}"));
        assert!(chain.add_style_from_parts("root", 2, "layer3", "style3{color:green}"));
        // adding same element should return false
        assert!(!chain.add_style_from_parts("root", 2, "layer3", "style3{color:green}"));

        let representation = chain
            .styles
            .get("root")
            .unwrap()
            .render(false, "root")
            .unwrap();
        let mut expectation = String::from("@layer root,layer2,layer3;");
        expectation.push_str("@layer root{style1{color:red}}");
        expectation.push_str("@layer layer2{style2{color:blue}}");
        expectation.push_str("@layer layer3{style3{color:green}}");
        assert_eq!(representation, expectation);
    }

    #[test]
    fn test_chain() {
        rcss::css! {
            @rcss(pub struct Style1);
            .foo{color:red}
        }
        rcss::css! {
            @rcss(pub struct Style2);
            @rcss(extend Style1);
            .foo{color:orange}
        }
        rcss::css! {
            @rcss(pub struct Style3);
            @rcss(extend Style2);
            .foo{color:green}
        }
        let mut chain = LayeredCss::new();
        assert!(chain.add_style_chain(&Style3::new()));
        let root_id = Style1::SCOPE_ID;
        let layer2_id = Style2::SCOPE_ID;
        let layer3_id = Style3::SCOPE_ID;
        let root_foo = Style1::new().foo;
        let layer2_foo = Style2::new().foo.split_whitespace().last().unwrap();
        let layer3_foo = Style3::new().foo.split_whitespace().last().unwrap();

        let mut expectation = format!("@layer {root_id},{layer2_id},{layer3_id};");

        expectation.push_str(&format!("@layer {root_id}{{.{root_foo}{{color:red}}}}"));
        expectation.push_str(&format!(
            "@layer {layer2_id}{{.{layer2_foo}{{color:orange}}}}"
        ));
        expectation.push_str(&format!(
            "@layer {layer3_id}{{.{layer3_foo}{{color:green}}}}"
        ));

        assert_eq!(
            chain
                .styles
                .get(root_id)
                .unwrap()
                .render(false, root_id)
                .unwrap(),
            expectation
        );
    }
}
