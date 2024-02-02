use std::{collections::BTreeMap, rc::Rc};

use leptos::{
    component, create_effect, create_rw_signal, leptos_dom::logging::console_log, provide_context,
    use_context, IntoView, RwSignal, SignalGet, SignalUpdate,
};
use leptos_meta::use_head;
use rcss::{CssCommon, CssWithStyle};

pub mod private;

/// Entry point to register your inline style.
/// Uses `CssWithStyle` as input.
///
/// Internally register basic class, and one that extends it.
///
pub fn register_inline_style<T>(css: &CssWithStyle<T>)
where
    T: CssCommon,
{
    //1. add basic class
    crate::private::append_layer_effect(T::BASIC_SCOPE, T::BASIC_SCOPE, T::BASIC_STYLE.to_string());
    //2. add extension class
    crate::private::append_layer_effect(
        T::BASIC_SCOPE,
        css.scoped_class(),
        css.style().to_string(),
    );
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
