use std::{cell::RefCell, rc::Rc};

use leptos::{
    component, create_effect, create_rw_signal, provide_context, use_context, IntoView, Oco,
    RwSignal, SignalUpdate, SignalUpdateUntracked,
};
use leptos_meta::use_head;

#[derive(Clone)]
pub struct LayeredCssMeta {
    pub layered_style: Rc<RwSignal<rcss_layers::LayeredCss>>,
}
impl LayeredCssMeta {
    pub fn new() -> Self {
        Self {
            layered_style: Rc::new(create_rw_signal(rcss_layers::LayeredCss::new())),
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

pub fn append_layer_effect(
    root_scope_id: impl Into<rcss_layers::ScopeId> + 'static,
    layer_scope_id: impl Into<rcss_layers::ScopeId> + 'static,
    order: rcss_layers::Order,
    style: impl Into<rcss_layers::Style> + 'static,
) {
    let run_once = run_once(move || {
        let style = style.into();
        if style.is_empty() {
            return;
        }
        let root_scope_id = root_scope_id.into();
        let layer_scope_id = layer_scope_id.into();
        let mut was_changed = false;
        let layered_css_meta = LayeredCssMeta::get();

        let was_changed_ref = &mut was_changed;
        layered_css_meta
            .layered_style
            .update_untracked(move |layers| {
                // use returned value to marker that signal should be updated
                *was_changed_ref |=
                    layers.add_style_from_parts(root_scope_id, order, layer_scope_id, style);
            });
        if was_changed {
            layered_css_meta.layered_style.update(|_| {});
        }
    });

    // We need to call this method only on client
    create_effect(move |_| run_once());
}

pub(crate) fn run_once<F>(func: F) -> impl Fn() + 'static
where
    F: FnOnce() + 'static,
{
    let run_once = RefCell::new(Some(func));
    move || {
        run_once.borrow_mut().take().map(|f| f());
    }
}

#[component]
pub(crate) fn custom_style(
    id: rcss_layers::ScopeId,
    style: rcss_layers::StyleTree,
) -> impl IntoView {
    let meta = use_head();

    let builder_el = leptos::leptos_dom::html::as_meta_tag({
        || {
            leptos::leptos_dom::html::style()
                .attr("id", id.clone())
                .attr("nonce", leptos::nonce::use_nonce())
        }
    });

    let builder_el = if !style.uniq_layers.is_empty() {
        match style.render(false, id.clone()) {
            Some(css) => builder_el.child(css),
            None => builder_el,
        }
    } else {
        builder_el
    };

    let id: Oco<'static, str> = id.into();
    meta.tags.register(id.into(), builder_el.into_any());
}
