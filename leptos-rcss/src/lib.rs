use leptos::{component, IntoView, SignalGet};

use rcss::extend::in_chain_ops::ScopeChainOps;

pub mod private;

pub type Scope = &'static str;
pub type Style = &'static str;

///
/// Entry point to register your inline styles.
/// Collects all styles and register them as seperate css @layer.
///
/// Each layer name is a scope id with number prefix.
///
/// TODO: It will do nothing if rcss uses file boundiling.
///

pub fn register_style_chain_as_layers<T>(chain: &T)
where
    T: ScopeChainOps,
{
    let root_scope_id = chain.root_scope_id();
    let mut scoped_styles = vec![];

    chain.for_each(|scope_id, style| {
        scoped_styles.push((scope_id, style));
    });

    for (order, (scope_id, style)) in scoped_styles.iter().rev().enumerate() {
        if *scope_id == root_scope_id {
            debug_assert_eq!(order, 0);
        }
        private::append_layer_effect(root_scope_id, scope_id, (order as u32, style.to_string()));
    }
}

#[component]
pub fn render_css() -> impl IntoView {
    use leptos::For;
    use private::CustomStyle;

    let styles = crate::private::LayeredCssMeta::get().styles.split().0;

    leptos::view! {
            <For each= move|| styles.get()
                key = |(key,v)| (*key, v.layers.len())
                let:css
                >
                    <CustomStyle id=css.0 style =css.1 />
            </For>
    }
}
