use std::{collections::BTreeMap, rc::Rc};

use leptos::{
    component, create_effect, create_rw_signal, leptos_dom::logging::console_log, provide_context,
    use_context, IntoView, RwSignal, SignalUpdate,
};
use leptos_meta::use_head;

#[derive(Clone)]
pub struct LayeredCssMeta {
    pub styles: Rc<RwSignal<BTreeMap<StyleId, LayeredCss>>>,
}
impl LayeredCssMeta {
    pub fn new() -> Self {
        Self {
            styles: Rc::new(create_rw_signal(BTreeMap::new())),
        }
    }
    pub fn get() -> Self {
        match use_context::<LayeredCssMeta>() {
            None => {
                let meta = LayeredCssMeta::new();
                provide_context(meta.clone());
                meta
            }
            Some(ctx) => ctx,
        }
    }
}
pub type StyleId = &'static str;
pub type Order = u32;
/// Store chain of extensions.
/// Order of layers is determined by how far a layer is from root.
#[derive(Default, Clone, Debug)]
pub struct LayeredCss {
    // Style by id of its layer
    pub layers: BTreeMap<StyleId, (Order, String)>,
}

pub fn append_layer_effect(
    css_uniq_class: StyleId,
    new_layer: StyleId,
    layer_impl: (Order, String),
) {
    if layer_impl.1.is_empty() {
        return;
    }
    create_effect(move |_| {
        let layered_css_meta = LayeredCssMeta::get();
        let css_uniq_class = css_uniq_class;
        let new_layer = new_layer;
        let layer_impl = layer_impl.clone();
        layered_css_meta.styles.update(move |style| {
            style
                .entry(css_uniq_class)
                .or_insert_with(Default::default)
                .layers
                // TODO: assert if on same layer received different order
                .insert(new_layer, layer_impl);
        });
    });
}

#[component]
pub(crate) fn custom_style(id: StyleId, style: LayeredCss) -> impl IntoView {
    let meta = use_head();

    let builder_el = leptos::leptos_dom::html::as_meta_tag({
        move || {
            leptos::leptos_dom::html::style()
                .attr("id", id)
                .attr("nonce", leptos::nonce::use_nonce())
        }
    });

    let builder_el = if !style.layers.is_empty() {
        let default_layer = style.layers.get(id).cloned();
        let mut rest = style.layers.iter().filter(|(layer, _)| **layer != id);
        // No default layer registered, just return style without layers
        if default_layer.is_none() {
            return meta.tags.register(id.into(), builder_el.into_any());
        }

        if rest.next().is_none() {
            // if no other layers just return plain css without layers.
            return meta.tags.register(
                id.into(),
                builder_el.child(default_layer.unwrap().1).into_any(),
            );
        }

        let mut ordered_layers: Vec<_> = style
            .layers
            .into_iter()
            .map(|(layer, (order, style))| (order, layer, style))
            .collect();
        console_log(format!("ordered_layers {:?}", &ordered_layers).as_str());
        ordered_layers.sort_by_key(|(order, _, _)| *order);
        // Make sure that default layer is always first
        debug_assert_eq!(ordered_layers.first().unwrap().1, id);

        let mut header = ordered_layers.iter().map(|(_, scope_id, _)| scope_id).fold(
            String::from("@layer "),
            |mut header, scope_id| {
                header.push_str(scope_id);
                header.push_str(",");
                header
            },
        );
        // just replace last comma with semicolon
        header.replace_range((header.len() - 1).., ";");

        let mut body = String::new();

        // Push all layer declaration to header
        for (_, scope_id, layer_impl) in ordered_layers {
            // push each individual to a body
            body.push_str(format!("@layer {} {{", scope_id).as_str());
            body.push_str(layer_impl.as_str());
            body.push_str("}");
        }
        let style = [header, body].concat();

        builder_el.child(style)
    } else {
        builder_el
    };

    meta.tags.register(id.into(), builder_el.into_any());
}
