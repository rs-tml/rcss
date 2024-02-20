mod my_button;

fn main() {
    let default_button = component_library::default_button("DefaultButton".to_string());
    let button =
        component_library::button("Button1".to_string(), None::<component_library::Button>);
    let my_button = my_button::my_button("MyButton".to_string());

    // since we using rcss.disable-styles = true in Cargo.toml
    // Styles in the component_library::button::Button::STYLE and my_button::MyButtonCss::STYLE are not generated.

    let default_style = <component_library::button::Button as rcss::ScopeCommon>::STYLE;
    assert!(default_style.is_empty());

    let html = format!(
        "<div>
            ButtonWithNone: {button}
        </div>
        <div>
            MyButton: {my_button}
        </div>
        <div>
            DefaultButton: {default_button}
        </div>"
    );

    let style = include_str!("../styles/style.css");
    println!("<style>{}</style>\n{}", style, html);
}
