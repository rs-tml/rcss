use leptos::*;

rcss::css! {
    @rcss(pub struct Css);
        .button {
            background-color: blue;
            color: white;
            border: none;
            padding: 15px 32px;
            text-align: center;
            text-decoration: none;
            display: inline-block;
            font-size: 16px;
            margin: 4px 2px;
            cursor: pointer;
        }

        .view{
            font-size: 20px;
            background-color: #f1f1f1;
            margin: 0 20px;
        }
}

use rcss::extend::{in_chain_ops::ScopeChainOps, StyleChain};

#[component]
pub fn Counter(#[prop(optional)] css: Option<StyleChain<Css>>) -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    let class = css.unwrap_or_else(Default::default);
    // console_log(format!("{:?}", &class).as_str());
    rcss_leptos::register_styles(class.clone());
    let scoped_classes = class.get_all_scopes().join(" ");
    let cm = |class: &str| format!("{scoped_classes} {class}");
    view! {

        <button class={cm(class.button)} on:click=on_click>"Increase"</button>
        <span class={cm(class.view)}>"Counter: "{count}</span>
    }
}

#[component]
pub fn CounterPage() -> impl IntoView {
    let class = rcss::css! {
        .link {
            background-color: white;
            color: #000;
            text-decoration: none;
            &:hover {
                text-decoration: underline;
            }
        }
    };

    rcss_leptos::register_styles(class.clone());
    view! {
        <Counter/>
        <a href="/" class=class.link >"Go to customized counter"</a>
    }
}
