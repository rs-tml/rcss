use leptos::{
    component, create_effect, Children, IntoView, SignalGet, SignalUpdate, SignalUpdateUntracked,
};

use rcss::extend::in_chain_ops::ScopeChainOps;

pub mod private;

pub type Scope = &'static str;
pub type Style = &'static str;

///
/// Entry point to register your inline styles.
/// Collects all styles and register them as seperate css @layer.
/// Each layer name is a scope id.
///
/// Notifies `RenderView` component to update styles in the DOM.
/// It will do nothing if style is empty or already registered.
///
/// Check out rcss-bundle for usage with style bundling.
///
///
/// # Example:
/// ```no_build
/// use leptos::*;
///
/// #[component]
/// fn my_component() -> impl IntoView {
///   let style = rcss::css!{
///       .my-class {
///          color: red;
///         & .nested-class {
///            color: blue;
///        }
///  };
///  rcss_leptos::register_styles(style.clone());
///  view! {
///     <div class=style.my_class>
///       "Hello "<span class=style.nested_class>"world"</span>
///     </div>
///  }
/// }
/// ```
///
///

pub fn register_styles<T>(chain: T)
where
    T: ScopeChainOps + 'static,
{
    let run_once = crate::private::run_once(move || {
        let meta = private::LayeredCssMeta::get();
        let mut was_changed = false;
        meta.layered_style
            .update_untracked(|l| was_changed |= l.add_style_chain(&chain));

        if was_changed {
            meta.layered_style.update(|_| {});
        }
    });
    create_effect(move |_| run_once());
}
///
/// Provide a context to use rcss styles.
/// In reality it is a pseudo component that doesn't render anything.
/// But it uses meta context to register new <style> elements.
///
/// # Example:
/// ```no_build
/// use leptos::*;
/// use rcss_leptos::WithStyles;
///
///
/// #[component]
/// fn app() -> impl IntoView {
///  view!{
///     <WithStyles>
///      <RealEntrypoint/>
///     </WithStyles>
///  }
/// }
/// ```
///
#[component]
pub fn with_styles(children: Children) -> impl IntoView {
    use leptos::For;
    use private::CustomStyle;

    let styles = crate::private::LayeredCssMeta::get()
        .layered_style
        .split()
        .0;
    leptos::view! {
        <For each= move|| styles.get().styles.into_iter()
            key = |v| (v.0.clone(), v.1.uniq_layers.len())
            let:css
            >
                <CustomStyle id=css.0 style =css.1 />
        </For>
        {children()}
    }
}
