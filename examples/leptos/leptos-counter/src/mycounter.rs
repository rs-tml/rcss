use crate::counter::Counter as CustomizableCounter;
use leptos::*;

rcss::css! {
    @rcss(pub struct Css);
    @rcss(extend crate::counter::Css);
    .button {
        background-color: green;

    }

    /* I can modify even type */
    span {
        background-color: red;
    }
}

#[component]
pub fn Counter() -> impl IntoView {
    // Creates a reactive value to update the button

    view! {
        <CustomizableCounter css=Css::new().into() />
    }
}

#[component]
pub fn MyCounterPage() -> impl IntoView {
    view! {
        <Counter/>
        <a href="/default">"Go to default"</a>
    }
}
