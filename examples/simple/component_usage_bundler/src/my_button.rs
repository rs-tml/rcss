use rcss::css;
css! {
    @rcss(pub struct MyButtonCss);
    /* not currently only full path is supported */
    @rcss(extend ::component_library::button::Button);
    .button {
        background-color: green;
    }
}

// Extending component
pub fn my_button(text: String) -> String {
    component_library::button(text, Some(MyButtonCss::new()))
}
