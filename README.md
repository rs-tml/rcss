# RCSS - Macro that embed CSS into Rust app.

## Motivation: 
There is two ways of writing function-macros in rust.
- One is to handle `TokenStream` from proc-macro.
This way saves link to original code, therefore IDE and compiller can show errors linked to original code, variables can be resolved and so on. But `TokenStream` in Rust is specialized for Rust syntax and it's harder to support foreign syntax in it.

Check out [rstml::RawText](https://github.com/rs-tml/rstml/blob/main/src/node/raw_text.rs) or [rstml::NodeName](https://github.com/rs-tml/rstml/blob/main/src/node/node_name.rs) both of this
structs are providing a hacky way to parse HTML-specific syntax, like dash-seperated-idents or unquoted text.

- The other way is to handle macro input as regular string.
This way is more flexible, because you can use any parser you want, but you loose all benefits of `TokenStream` and have to write your own parser.

Unlike HTML-templating where you need some way to mix reactive variables from Rust and templates -
CSS usually contain static predefined content, which rarely need to be generated at runtime or compile time.
Therefore link between original code and IDE is less important. Instead most of the users just want to write CSS for their components near it's implementation.

So instead of writing custom parser on top of `proc-macro::TokenStream`, this library tryies to convert macro call into string and work with convenient css preprocessors.

## Info:

RCSS support various embedding modes:
- Can preprocess css and output struct that work like css_modules.
- Can output class-name for scoped-style api (this class name should be injected to all elements).

Support multiple backends:
- [stylers](https://github.com/abishekatp/stylers) - library that provide css-like syntax, and aimed for `leptos` framework. Has smaller dependencies footprint, but slower and less powerfull.

- [lightningcss](https://lightningcss.dev/) - library from Parcel, writed on top of browsergrade `cssparser` - extremely fast and powerfull. But has large dependencies footprint.

- [procss](https://github.com/ProspectiveCo/procss) (in plans) - simple nom-based css parser, and preprocessor. Has small amount of dependencies, but work slower than `lightningcss`, and doesn't support all css syntax.

Can output inline css content or agregate all css into one file (using build.rs).
Contain various helpers for macro and build.rs writing.

## Usage:

Lib contain feature flag for each backend, so you can choose which one to use.

```toml
[dependencies]
rcss = { version = "0.1", features = ["lightningcss"] }
```
This allows optimize compiliation time, and reduce dependencies footprint on demand.

All api is devided into modules, and can be represented as a tree:
```text
--- rcss
    |- inline  (api that return css as string in second output params)
    |   |- css_modules 
    |   |- scoped
    |- file  (api that doesn't return css as string, but provide a way to agregate them into file using build.rs)
        |- css_modules
        |- scoped
```
Macros in this library can't be used from other macros, because they need access to source code.
Macros in `file` should be used in pair with build.rs helper. 

Scoped css is inspired by vue, other js frameworks and shadow dom, in vue it uses custom attribute and css preprocessor in order to scope css rules.
Our aproach uses class names, that should be attached to each styled html element.
It can be automaticaly injected by framework, or manually by user.
Scoped api contain single macro `css!` that can be used to generate scoped css class name.
It return tuple of `(css_class, inline_css)` on inline api and `(css_class)` on file api.
`css_class` is a string that should be injected to each styled html element.
`inline_css` is a string that should be injected to `<style>` or in other way delivered to the client as css content.

`css_modules` is familiar for js developers, it dynamicly chages each class-name, and provide a single object with original class names as fields, and new class names as values.
The same aproach is aplied in rust, but instead of object, it return newly generated struct.
Css modules api contain two macros `css!` and `css_struct!`.
`css!` macro is work in simmilar way as scoped api, but instead of `class_name` it return struct which fields can be used to style html elements.
 `css_struct!` is used to define css struct in global context.


## Examples: 

RCSS can be used without frameworks at all, example using scoped-style api:
```rust
use rcss::inline::scoped::css;

fn main() {
    let (css_class, style) = css!{
        .container {
            background-color: black;
        }
    };
    
    let html = format!(r#"<div class="{} my-class">Hello</div>"#, css_class);
    let html = format!(r#"<style>{}</style>{}"#, style, html);
    // output html.
}

```

It's easy to use it with any framework,
example for `leptos` framework, using `css_modules` api:

```rust

use leptos::prelude::*;
use rcss::inline::css_modules::css;

#[component]
fn some_component() -> impl View {
    let (css_class, inline_css) = css!{
        .container {
            background-color: black;
        }
    };
    
    view! {
        <div class={css_class.container}>
            Hello
        </div>
        <style>
            {inline_css}
        </style>

    }
}

```

Leptos also support scoped-style api trough class injecting, so more complex example using scoped-api can be written as:

build.rs:
```rust
use rcss::builer_helper::process_styles_to_file;
fn main () {
    let project_path = std::env!("CARGO_MANIFEST_DIR");
    let output = format!("{}/target/styles.rs", project_path);
    process_styles_to_file(project_path, output);
}
```

component.rs:
```rust
use leptos::prelude::*;
use rcss::file::scoped::css;

#[component]
fn some_component() -> impl View {
    let css_class = css!{
        .container {
            background-color: black;
        }
    };
    
    view! {
        <div class={css_class}>
            Hello
        </div>

    }
}

```

