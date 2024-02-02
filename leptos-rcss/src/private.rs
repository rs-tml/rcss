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

#[derive(Default, Clone, Debug)]
pub struct LayeredCss {
    // Style by id of its layer
    pub layers: BTreeMap<StyleId, String>,
}

pub fn append_layer_effect(css_uniq_class: StyleId, new_layer: StyleId, layer_impl: String) {
    create_effect(move |_| {
        let layered_css_meta = LayeredCssMeta::get();
        let css_uniq_class = css_uniq_class;
        let new_layer = new_layer;
        let layer_impl = layer_impl.clone();
        console_log(
            format!(
                "Appending layer {} to {}, style ={} ",
                new_layer, css_uniq_class, layer_impl
            )
            .as_str(),
        );
        layered_css_meta.styles.update(move |style| {
            style
                .entry(css_uniq_class)
                .or_insert_with(Default::default)
                .layers
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
        // make sure that default unique class is layer 0 (lesser priority)
        let default_layer = style.layers.get(id).cloned().unwrap_or("".to_string());
        let rest = style.layers.into_iter().filter(|(layer, _)| *layer != id);
        let layer_iter = Some((id, default_layer)).into_iter().chain(rest);

        let mut header = String::new();
        let mut body = String::new();
        header.push_str("@layer ");
        let not_default = |header: &str| header.len() > 7;
        // Push all layer declaration to header
        for (layer, layer_impl) in layer_iter {
            console_log(
                format!(
                    "Processing layer {} in {}, style ={} ",
                    layer, id, layer_impl
                )
                .as_str(),
            );
            // Push all layer declaration to header
            if not_default(&header) {
                header.push_str(",");
            }
            header.push_str(layer);

            // push each individual to a body
            body.push_str(format!("@layer {} {{\n", layer).as_str());
            body.push_str(layer_impl.as_str());
            body.push_str("}\n");
        }
        header.push_str(";\n");
        let style = [header, body].concat();

        builder_el.child(style)
    } else {
        builder_el
    };

    meta.tags.register(id.into(), builder_el.into_any());
}
