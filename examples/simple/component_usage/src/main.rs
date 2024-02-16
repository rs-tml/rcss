mod my_button;
use rcss::ScopeCommon;

fn main() {
    let default_button = component_library::default_button("DefaultButton".to_string());
    let button =
        component_library::button("Button1".to_string(), None::<component_library::Button>);
    let my_button = my_button::my_button("MyButton".to_string());

    let default_style = component_library::button::Button::STYLE;
    let my_style = my_button::MyButtonCss::STYLE;
    let style = format!(
        "<style>
            {default_style}
            {my_style}
        </style>",
        default_style = default_style,
        my_style = my_style
    );

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
    println!("{}\n{}", style, html);
}
